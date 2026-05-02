use crate::backend::InferenceBackend;
use crate::error::{InferenceError, InferenceResult};
use crate::image::{SdImage, SdVideo};
use crate::params::*;
use crate::types::*;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CliBackendConfig {
    pub sd_cli_path: String,
    pub n_threads: i32,
    pub flash_attn: bool,
    pub offload_to_cpu: bool,
    pub clip_on_cpu: bool,
    pub vae_on_cpu: bool,
    pub mmap: bool,
    pub verbose: bool,
    pub default_output_dir: String,
}

impl Default for CliBackendConfig {
    fn default() -> Self {
        Self {
            sd_cli_path: "sd-cli".to_string(),
            n_threads: -1,
            flash_attn: false,
            offload_to_cpu: false,
            clip_on_cpu: false,
            vae_on_cpu: false,
            mmap: false,
            verbose: false,
            default_output_dir: "output".to_string(),
        }
    }
}

impl CliBackendConfig {
    pub fn new(sd_cli_path: impl Into<String>) -> Self {
        Self {
            sd_cli_path: sd_cli_path.into(),
            ..Default::default()
        }
    }

    pub fn with_threads(mut self, n: i32) -> Self {
        self.n_threads = n;
        self
    }

    pub fn with_flash_attn(mut self, enable: bool) -> Self {
        self.flash_attn = enable;
        self
    }

    pub fn with_offload_to_cpu(mut self, enable: bool) -> Self {
        self.offload_to_cpu = enable;
        self
    }

    pub fn with_clip_on_cpu(mut self, enable: bool) -> Self {
        self.clip_on_cpu = enable;
        self
    }

    pub fn with_vae_on_cpu(mut self, enable: bool) -> Self {
        self.vae_on_cpu = enable;
        self
    }

    pub fn with_verbose(mut self, enable: bool) -> Self {
        self.verbose = enable;
        self
    }

    pub fn with_output_dir(mut self, dir: impl Into<String>) -> Self {
        self.default_output_dir = dir.into();
        self
    }
}

pub struct CliBackend {
    config: CliBackendConfig,
}

impl CliBackend {
    pub fn new(config: CliBackendConfig) -> InferenceResult<Self> {
        let cli_path = Path::new(&config.sd_cli_path);
        if cli_path.exists() {
            Self::ensure_executable(cli_path);
        } else {
            let which_output = Command::new("which")
                .arg(&config.sd_cli_path)
                .output()
                .ok();
            if which_output.is_none() || !which_output.unwrap().status.success() {
                tracing::warn!(
                    "sd-cli not found at '{}', CLI backend may not work",
                    config.sd_cli_path
                );
            }
        }
        Ok(Self { config })
    }

    fn ensure_executable(path: &Path) {
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            if let Ok(metadata) = path.metadata() {
                let mut perms = metadata.permissions();
                let mode = perms.mode();
                if (mode & 0o111) == 0 {
                    perms.set_mode(mode | 0o755);
                    if let Err(e) = std::fs::set_permissions(path, perms) {
                        tracing::warn!("Failed to set execute permission on {:?}: {}", path, e);
                    } else {
                        tracing::info!("Set execute permission on {:?}", path);
                    }
                }
            }
        }

        #[cfg(target_os = "macos")]
        {
            let _ = Command::new("xattr")
                .args(["-cr", &path.to_string_lossy()])
                .output();
        }
    }

    fn build_context_args(&self, model_config: &ModelConfig, args: &mut Vec<String>) {
        if let Some(ref path) = model_config.model_path {
            args.push("-m".to_string());
            args.push(path.clone());
        }
        if let Some(ref path) = model_config.clip_l_path {
            args.push("--clip_l".to_string());
            args.push(path.clone());
        }
        if let Some(ref path) = model_config.clip_g_path {
            args.push("--clip_g".to_string());
            args.push(path.clone());
        }
        if let Some(ref path) = model_config.clip_vision_path {
            args.push("--clip_vision".to_string());
            args.push(path.clone());
        }
        if let Some(ref path) = model_config.t5xxl_path {
            args.push("--t5xxl".to_string());
            args.push(path.clone());
        }
        if let Some(ref path) = model_config.llm_path {
            args.push("--llm".to_string());
            args.push(path.clone());
        }
        if let Some(ref path) = model_config.llm_vision_path {
            args.push("--llm_vision".to_string());
            args.push(path.clone());
        }
        if let Some(ref path) = model_config.diffusion_model_path {
            args.push("--diffusion-model".to_string());
            args.push(path.clone());
        }
        if let Some(ref path) = model_config.vae_path {
            args.push("--vae".to_string());
            args.push(path.clone());
        }
        if let Some(ref path) = model_config.control_net_path {
            args.push("--control-net".to_string());
            args.push(path.clone());
        }
    }

    fn build_config_args(&self, args: &mut Vec<String>) {
        if self.config.n_threads > 0 {
            args.push("-t".to_string());
            args.push(self.config.n_threads.to_string());
        }
        if self.config.flash_attn {
            args.push("--fa".to_string());
        }
        if self.config.offload_to_cpu {
            args.push("--offload-to-cpu".to_string());
        }
        if self.config.clip_on_cpu {
            args.push("--clip-on-cpu".to_string());
        }
        if self.config.vae_on_cpu {
            args.push("--vae-on-cpu".to_string());
        }
        if self.config.mmap {
            args.push("--mmap".to_string());
        }
        if self.config.verbose {
            args.push("-v".to_string());
        }
    }

    fn build_sample_args(&self, params: &SampleParams, args: &mut Vec<String>) {
        args.push("--steps".to_string());
        args.push(params.sample_steps.to_string());

        args.push("--sampling-method".to_string());
        args.push(format!("{}", params.sample_method));

        args.push("--scheduler".to_string());
        args.push(format!("{}", params.scheduler));

        args.push("--cfg-scale".to_string());
        args.push(format!("{}", params.guidance.txt_cfg));

        if params.guidance.img_cfg.is_some() {
            args.push("--img-cfg-scale".to_string());
            args.push(format!("{}", params.guidance.img_cfg.unwrap()));
        }

        if params.guidance.distilled_guidance != 3.5 {
            args.push("--guidance".to_string());
            args.push(format!("{}", params.guidance.distilled_guidance));
        }

        if let Some(eta) = params.eta {
            args.push("--eta".to_string());
            args.push(format!("{}", eta));
        }

        if params.shifted_timestep > 0 {
            args.push("--timestep-shift".to_string());
            args.push(params.shifted_timestep.to_string());
        }

        if let Some(flow_shift) = params.flow_shift {
            args.push("--flow-shift".to_string());
            args.push(format!("{}", flow_shift));
        }
    }

    fn run_sd_cli(&self, args: &[String]) -> InferenceResult<std::process::Output> {
        tracing::debug!("Running: {} {}", self.config.sd_cli_path, args.join(" "));

        let cli_path = Path::new(&self.config.sd_cli_path);
        if cli_path.exists() {
            Self::ensure_executable(cli_path);
        }

        let output = Command::new(&self.config.sd_cli_path)
            .args(args)
            .output()
            .map_err(|e| {
                if e.kind() == std::io::ErrorKind::PermissionDenied {
                    InferenceError::BackendNotAvailable(format!(
                        "Permission denied executing sd-cli at '{}'. Try: chmod +x '{}' or xattr -cr '{}'",
                        self.config.sd_cli_path, self.config.sd_cli_path, self.config.sd_cli_path
                    ))
                } else {
                    InferenceError::BackendNotAvailable(format!(
                        "Failed to execute sd-cli: {}", e
                    ))
                }
            })?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);

        if self.config.verbose || !output.status.success() {
            if !stdout.is_empty() {
                tracing::debug!("sd-cli stdout: {}", stdout);
            }
            if !stderr.is_empty() {
                if output.status.success() {
                    tracing::debug!("sd-cli stderr: {}", stderr);
                } else {
                    tracing::error!("sd-cli stderr: {}", stderr);
                }
            }
        }

        if !output.status.success() {
            return Err(InferenceError::GenerationFailed(format!(
                "sd-cli exited with status {}: {}",
                output.status,
                stderr.lines().take(5).collect::<Vec<_>>().join("\n")
            )));
        }

        Ok(output)
    }
}

impl InferenceBackend for CliBackend {
    fn supports_image_generation(&self) -> bool {
        Path::new(&self.config.sd_cli_path).exists()
            || Command::new("which")
                .arg(&self.config.sd_cli_path)
                .output()
                .map(|o| o.status.success())
                .unwrap_or(false)
    }

    fn supports_video_generation(&self) -> bool {
        self.supports_image_generation()
    }

    fn generate_image(&self, params: ImageGenParams) -> InferenceResult<Vec<SdImage>> {
        let mut args = Vec::new();
        args.push("-M".to_string());
        args.push("img_gen".to_string());

        self.build_context_args(&params.model_config, &mut args);
        self.build_config_args(&mut args);
        self.build_sample_args(&params.sample_params, &mut args);

        args.push("-p".to_string());
        args.push(params.prompt.clone());

        if !params.negative_prompt.is_empty() {
            args.push("-n".to_string());
            args.push(params.negative_prompt.clone());
        }

        args.push("-W".to_string());
        args.push(params.width.to_string());
        args.push("-H".to_string());
        args.push(params.height.to_string());

        args.push("-s".to_string());
        args.push(params.seed.to_string());

        args.push("-b".to_string());
        args.push(params.batch_count.to_string());

        if params.clip_skip > 0 {
            args.push("--clip-skip".to_string());
            args.push(params.clip_skip.to_string());
        }

        if params.strength != 0.75 {
            args.push("--strength".to_string());
            args.push(format!("{}", params.strength));
        }

        if let Some(ref img) = params.init_image {
            let tmp_path = self.write_temp_image(img, "init")?;
            args.push("-i".to_string());
            args.push(tmp_path.to_string_lossy().to_string());
        }

        if let Some(ref mask) = params.mask_image {
            let tmp_path = self.write_temp_image(mask, "mask")?;
            args.push("--mask".to_string());
            args.push(tmp_path.to_string_lossy().to_string());
        }

        if let Some(ref ctrl_img) = params.control_image {
            let tmp_path = self.write_temp_image(ctrl_img, "control")?;
            args.push("--control-image".to_string());
            args.push(tmp_path.to_string_lossy().to_string());
            args.push("--control-strength".to_string());
            args.push(format!("{}", params.control_strength));
        }

        if params.vae_tiling_params.enabled {
            args.push("--vae-tiling".to_string());
        }

        if !params.loras.is_empty() {
            for lora in &params.loras {
                if !lora.is_high_noise {
                    let lora_prompt = format!("<lora:{}:{}>", lora.path, lora.multiplier);
                    if let Some(pos) = args.iter().position(|a| a == "-p") {
                        if let Some(prompt) = args.get_mut(pos + 1) {
                            prompt.push_str(&lora_prompt);
                        }
                    }
                }
            }
        }

        let output_dir = &self.config.default_output_dir;
        std::fs::create_dir_all(output_dir).ok();

        let output_path = Path::new(output_dir).join(format!("comfyui_cli_{}.png", params.seed));
        args.push("-o".to_string());
        args.push(output_path.to_string_lossy().to_string());

        self.run_sd_cli(&args)?;

        let mut images = Vec::new();
        if output_path.exists() {
            let img_data = std::fs::read(&output_path)
                .map_err(|e| InferenceError::ImageDecodeError(e.to_string()))?;
            let img = SdImage::from_png_bytes(&img_data)?;
            images.push(img);

            if !self.config.verbose {
                let _ = std::fs::remove_file(&output_path);
            }
        }

        if images.is_empty() {
            return Err(InferenceError::GenerationFailed(
                "sd-cli produced no output images".to_string(),
            ));
        }

        Ok(images)
    }

    fn generate_video(&self, params: VideoGenParams) -> InferenceResult<SdVideo> {
        let mut args = Vec::new();
        args.push("-M".to_string());
        args.push("vid_gen".to_string());

        self.build_context_args(&params.model_config, &mut args);
        self.build_config_args(&mut args);
        self.build_sample_args(&params.sample_params, &mut args);

        args.push("-p".to_string());
        args.push(params.prompt.clone());

        if !params.negative_prompt.is_empty() {
            args.push("-n".to_string());
            args.push(params.negative_prompt.clone());
        }

        args.push("-W".to_string());
        args.push(params.width.to_string());
        args.push("-H".to_string());
        args.push(params.height.to_string());

        args.push("-s".to_string());
        args.push(params.seed.to_string());

        args.push("--video-frames".to_string());
        args.push(params.video_frames.to_string());

        if let Some(ref img) = params.init_image {
            let tmp_path = self.write_temp_image(img, "init")?;
            args.push("-i".to_string());
            args.push(tmp_path.to_string_lossy().to_string());
        }

        if let Some(ref img) = params.end_image {
            let tmp_path = self.write_temp_image(img, "end")?;
            args.push("--end-img".to_string());
            args.push(tmp_path.to_string_lossy().to_string());
        }

        let output_dir = &self.config.default_output_dir;
        std::fs::create_dir_all(output_dir).ok();

        let output_path = Path::new(output_dir).join(format!("comfyui_cli_vid_{}.webm", params.seed));
        args.push("-o".to_string());
        args.push(output_path.to_string_lossy().to_string());

        self.run_sd_cli(&args)?;

        let frames = Vec::new();
        let video = SdVideo::new(frames, 16);

        Ok(video)
    }

    fn upscale(&self, image: SdImage, params: UpscaleParams) -> InferenceResult<SdImage> {
        let mut args = Vec::new();
        args.push("-M".to_string());
        args.push("upscale".to_string());

        args.push("--upscale-model".to_string());
        args.push(params.esrgan_path.clone());

        if params.offload_to_cpu {
            args.push("--offload-to-cpu".to_string());
        }

        if params.n_threads > 0 {
            args.push("-t".to_string());
            args.push(params.n_threads.to_string());
        }

        args.push("--upscale-tile-size".to_string());
        args.push(params.tile_size.to_string());

        let input_path = self.write_temp_image(&image, "upscale_input")?;
        args.push("-i".to_string());
        args.push(input_path.to_string_lossy().to_string());

        let output_dir = &self.config.default_output_dir;
        std::fs::create_dir_all(output_dir).ok();

        let output_path = Path::new(output_dir).join(format!("comfyui_cli_upscale_{}.png", image.width));
        args.push("-o".to_string());
        args.push(output_path.to_string_lossy().to_string());

        if self.config.verbose {
            args.push("-v".to_string());
        }

        self.run_sd_cli(&args)?;

        if output_path.exists() {
            let img_data = std::fs::read(&output_path)
                .map_err(|e| InferenceError::ImageDecodeError(e.to_string()))?;
            let img = SdImage::from_png_bytes(&img_data)?;
            Ok(img)
        } else {
            Err(InferenceError::GenerationFailed(
                "sd-cli upscale produced no output".to_string(),
            ))
        }
    }
}

impl CliBackend {
    fn write_temp_image(&self, image: &SdImage, prefix: &str) -> InferenceResult<PathBuf> {
        let tmp_dir = std::env::temp_dir().join("comfyui-rust");
        std::fs::create_dir_all(&tmp_dir).ok();

        let tmp_path = tmp_dir.join(format!("{}_{}.png", prefix, std::process::id()));
        let png_bytes = image.to_png_bytes()?;

        let mut file = std::fs::File::create(&tmp_path)?;
        file.write_all(&png_bytes)?;

        Ok(tmp_path)
    }
}

pub fn convert_model_cli(
    sd_cli_path: &str,
    params: ConvertParams,
) -> InferenceResult<bool> {
    let mut args = Vec::new();
    args.push("-M".to_string());
    args.push("convert".to_string());

    args.push("-m".to_string());
    args.push(params.input_path.clone());

    args.push("-o".to_string());
    args.push(params.output_path.clone());

    args.push("--type".to_string());
    args.push(format!("{}", params.output_type));

    if let Some(ref vae_path) = params.vae_path {
        args.push("--vae".to_string());
        args.push(vae_path.clone());
    }

    if let Some(ref rules) = params.tensor_type_rules {
        args.push("--tensor-type-rules".to_string());
        args.push(rules.clone());
    }

    if params.convert_name {
        args.push("--convert-name".to_string());
    }

    args.push("-v".to_string());

    let output = Command::new(sd_cli_path)
        .args(&args)
        .output()
        .map_err(|e| InferenceError::BackendNotAvailable(format!(
            "Failed to execute sd-cli: {}", e
        )))?;

    Ok(output.status.success())
}

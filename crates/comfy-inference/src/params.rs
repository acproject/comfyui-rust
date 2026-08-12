use crate::image::SdImage;
use crate::types::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Gaussian3DParams {
    pub model_config: ModelConfig,
    pub input_image: Option<SdImage>,
    pub seed: i64,
    pub steps: i32,
    pub guidance_scale: f32,
    pub num_gaussians: i32,
    pub erode_radius: i32,
    pub output_path: Option<String>,
    pub output_format: i32, // 0=PLY, 1=SPLAT
}

impl Default for Gaussian3DParams {
    fn default() -> Self {
        Self {
            model_config: ModelConfig::default(),
            input_image: None,
            seed: 42,
            steps: 20,
            guidance_scale: 3.0,
            num_gaussians: 262144,
            erode_radius: 1,
            output_path: None,
            output_format: 0,
        }
    }
}

impl Gaussian3DParams {
    pub fn new(model_config: ModelConfig) -> Self {
        Self {
            model_config,
            ..Default::default()
        }
    }

    pub fn with_input_image(mut self, image: SdImage) -> Self {
        self.input_image = Some(image);
        self
    }

    pub fn with_seed(mut self, seed: i64) -> Self {
        self.seed = seed;
        self
    }

    pub fn with_steps(mut self, steps: i32) -> Self {
        self.steps = steps;
        self
    }

    pub fn with_guidance_scale(mut self, scale: f32) -> Self {
        self.guidance_scale = scale;
        self
    }

    pub fn with_num_gaussians(mut self, n: i32) -> Self {
        self.num_gaussians = n;
        self
    }

    pub fn with_output_path(mut self, path: impl Into<String>) -> Self {
        self.output_path = Some(path.into());
        self
    }

    pub fn with_output_format(mut self, format: i32) -> Self {
        self.output_format = format;
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Gaussian3DOutput {
    pub xyz: Vec<f32>,
    pub features_dc: Vec<f32>,
    pub opacity: Vec<f32>,
    pub scaling: Vec<f32>,
    pub rotation: Vec<f32>,
    pub num_gaussians: usize,
    pub output_file: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextConfig {
    pub model_path: Option<String>,
    pub clip_l_path: Option<String>,
    pub clip_g_path: Option<String>,
    pub clip_vision_path: Option<String>,
    pub decoder_path: Option<String>,
    pub rmbg_path: Option<String>,
    pub t5xxl_path: Option<String>,
    pub llm_path: Option<String>,
    pub llm_vision_path: Option<String>,
    pub diffusion_model_path: Option<String>,
    pub high_noise_diffusion_model_path: Option<String>,
    pub vae_path: Option<String>,
    pub taesd_path: Option<String>,
    pub control_net_path: Option<String>,
    pub embeddings: Vec<EmbeddingEntry>,
    pub photo_maker_path: Option<String>,
    pub tensor_type_rules: Option<String>,
    pub vae_decode_only: bool,
    pub free_params_immediately: bool,
    pub n_threads: i32,
    pub wtype: SdType,
    pub rng_type: RngType,
    pub sampler_rng_type: Option<RngType>,
    pub prediction: Option<PredictionType>,
    pub lora_apply_mode: LoraApplyMode,
    pub offload_params_to_cpu: bool,
    pub enable_mmap: bool,
    pub multi_gpu: bool,
    pub keep_clip_on_cpu: bool,
    pub keep_control_net_on_cpu: bool,
    pub keep_vae_on_cpu: bool,
    pub flash_attn: bool,
    pub diffusion_flash_attn: bool,
    pub tae_preview_only: bool,
    pub diffusion_conv_direct: bool,
    pub vae_conv_direct: bool,
    pub circular_x: bool,
    pub circular_y: bool,
    pub text_encoder_path: Option<String>,
    pub embeddings_connectors_path: Option<String>,
    pub audio_vae_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingEntry {
    pub name: String,
    pub path: String,
}

impl Default for ContextConfig {
    fn default() -> Self {
        Self {
            model_path: None,
            clip_l_path: None,
            clip_g_path: None,
            clip_vision_path: None,
            decoder_path: None,
            rmbg_path: None,
            t5xxl_path: None,
            llm_path: None,
            llm_vision_path: None,
            diffusion_model_path: None,
            high_noise_diffusion_model_path: None,
            vae_path: None,
            taesd_path: None,
            control_net_path: None,
            embeddings: Vec::new(),
            photo_maker_path: None,
            tensor_type_rules: None,
            vae_decode_only: false,
            free_params_immediately: false,
            n_threads: -1,
            wtype: SdType::Auto,
            rng_type: RngType::Cuda,
            sampler_rng_type: None,
            prediction: None,
            lora_apply_mode: LoraApplyMode::Auto,
            offload_params_to_cpu: false,
            enable_mmap: false,
            multi_gpu: false,
            keep_clip_on_cpu: false,
            keep_control_net_on_cpu: false,
            keep_vae_on_cpu: false,
            flash_attn: false,
            diffusion_flash_attn: false,
            tae_preview_only: false,
            diffusion_conv_direct: false,
            vae_conv_direct: false,
            circular_x: false,
            circular_y: false,
            text_encoder_path: None,
            embeddings_connectors_path: None,
            audio_vae_path: None,
        }
    }
}

impl ContextConfig {
    pub fn new(model_path: impl Into<String>) -> Self {
        Self {
            model_path: Some(model_path.into()),
            ..Default::default()
        }
    }

    pub fn with_vae(mut self, vae_path: impl Into<String>) -> Self {
        self.vae_path = Some(vae_path.into());
        self
    }

    pub fn with_clip_l(mut self, path: impl Into<String>) -> Self {
        self.clip_l_path = Some(path.into());
        self
    }

    pub fn with_clip_g(mut self, path: impl Into<String>) -> Self {
        self.clip_g_path = Some(path.into());
        self
    }

    pub fn with_t5xxl(mut self, path: impl Into<String>) -> Self {
        self.t5xxl_path = Some(path.into());
        self
    }

    pub fn with_decoder(mut self, path: impl Into<String>) -> Self {
        self.decoder_path = Some(path.into());
        self
    }

    pub fn with_rmbg(mut self, path: impl Into<String>) -> Self {
        self.rmbg_path = Some(path.into());
        self
    }

    pub fn with_diffusion_model(mut self, path: impl Into<String>) -> Self {
        self.diffusion_model_path = Some(path.into());
        self
    }

    pub fn with_threads(mut self, n: i32) -> Self {
        self.n_threads = n;
        self
    }

    pub fn with_wtype(mut self, wtype: SdType) -> Self {
        self.wtype = wtype;
        self
    }

    pub fn with_flash_attn(mut self, enable: bool) -> Self {
        self.flash_attn = enable;
        self
    }

    pub fn with_offload_to_cpu(mut self, enable: bool) -> Self {
        self.offload_params_to_cpu = enable;
        self
    }

    pub fn with_multi_gpu(mut self, enable: bool) -> Self {
        self.multi_gpu = enable;
        self
    }

    pub fn with_mmap(mut self, enable: bool) -> Self {
        self.enable_mmap = enable;
        self
    }

    pub fn with_vae_decode_only(mut self, enable: bool) -> Self {
        self.vae_decode_only = enable;
        self
    }

    pub fn with_text_encoder(mut self, path: impl Into<String>) -> Self {
        self.text_encoder_path = Some(path.into());
        self
    }

    pub fn with_embeddings_connectors(mut self, path: impl Into<String>) -> Self {
        self.embeddings_connectors_path = Some(path.into());
        self
    }

    pub fn with_audio_vae(mut self, path: impl Into<String>) -> Self {
        self.audio_vae_path = Some(path.into());
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageGenParams {
    pub model_config: ModelConfig,
    pub loras: Vec<LoraEntry>,
    pub prompt: String,
    pub negative_prompt: String,
    pub clip_skip: i32,
    pub init_image: Option<SdImage>,
    pub ref_images: Vec<SdImage>,
    pub auto_resize_ref_image: bool,
    pub increase_ref_index: bool,
    pub mask_image: Option<SdImage>,
    pub width: i32,
    pub height: i32,
    pub sample_params: SampleParams,
    pub strength: f32,
    pub seed: i64,
    pub batch_count: i32,
    pub control_image: Option<SdImage>,
    pub control_strength: f32,
    pub vae_tiling_params: TilingParams,
    pub cache_params: CacheParams,
    pub hires_params: HiresParams,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelConfig {
    pub model_path: Option<String>,
    pub clip_l_path: Option<String>,
    pub clip_g_path: Option<String>,
    pub clip_vision_path: Option<String>,
    pub decoder_path: Option<String>,
    pub rmbg_path: Option<String>,
    pub t5xxl_path: Option<String>,
    pub llm_path: Option<String>,
    pub llm_vision_path: Option<String>,
    pub diffusion_model_path: Option<String>,
    pub vae_path: Option<String>,
    pub control_net_path: Option<String>,
    pub text_encoder_path: Option<String>,
    pub audio_vae_path: Option<String>,
    pub embeddings_connectors_path: Option<String>,
}

impl Default for ModelConfig {
    fn default() -> Self {
        Self {
            model_path: None,
            clip_l_path: None,
            clip_g_path: None,
            clip_vision_path: None,
            decoder_path: None,
            rmbg_path: None,
            t5xxl_path: None,
            llm_path: None,
            llm_vision_path: None,
            diffusion_model_path: None,
            vae_path: None,
            control_net_path: None,
            text_encoder_path: None,
            audio_vae_path: None,
            embeddings_connectors_path: None,
        }
    }
}

impl ModelConfig {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_model(mut self, path: impl Into<String>) -> Self {
        self.model_path = Some(path.into());
        self
    }

    pub fn with_clip_l(mut self, path: impl Into<String>) -> Self {
        self.clip_l_path = Some(path.into());
        self
    }

    pub fn with_clip_g(mut self, path: impl Into<String>) -> Self {
        self.clip_g_path = Some(path.into());
        self
    }

    pub fn with_clip_vision(mut self, path: impl Into<String>) -> Self {
        self.clip_vision_path = Some(path.into());
        self
    }

    pub fn with_decoder(mut self, path: impl Into<String>) -> Self {
        self.decoder_path = Some(path.into());
        self
    }

    pub fn with_rmbg(mut self, path: impl Into<String>) -> Self {
        self.rmbg_path = Some(path.into());
        self
    }

    pub fn with_t5xxl(mut self, path: impl Into<String>) -> Self {
        self.t5xxl_path = Some(path.into());
        self
    }

    pub fn with_llm(mut self, path: impl Into<String>) -> Self {
        self.llm_path = Some(path.into());
        self
    }

    pub fn with_llm_vision(mut self, path: impl Into<String>) -> Self {
        self.llm_vision_path = Some(path.into());
        self
    }

    pub fn with_diffusion_model(mut self, path: impl Into<String>) -> Self {
        self.diffusion_model_path = Some(path.into());
        self
    }

    pub fn with_vae(mut self, path: impl Into<String>) -> Self {
        self.vae_path = Some(path.into());
        self
    }

    pub fn with_control_net(mut self, path: impl Into<String>) -> Self {
        self.control_net_path = Some(path.into());
        self
    }

    pub fn with_text_encoder(mut self, path: impl Into<String>) -> Self {
        self.text_encoder_path = Some(path.into());
        self
    }

    pub fn with_audio_vae(mut self, path: impl Into<String>) -> Self {
        self.audio_vae_path = Some(path.into());
        self
    }

    pub fn with_embeddings_connectors(mut self, path: impl Into<String>) -> Self {
        self.embeddings_connectors_path = Some(path.into());
        self
    }

    pub fn cache_key(&self) -> String {
        let mut parts = Vec::new();
        if let Some(ref p) = self.model_path { parts.push(format!("m:{}", p)); }
        if let Some(ref p) = self.clip_l_path { parts.push(format!("cl:{}", p)); }
        if let Some(ref p) = self.clip_g_path { parts.push(format!("cg:{}", p)); }
        if let Some(ref p) = self.clip_vision_path { parts.push(format!("cv:{}", p)); }
        if let Some(ref p) = self.decoder_path { parts.push(format!("dec:{}", p)); }
        if let Some(ref p) = self.rmbg_path { parts.push(format!("rmbg:{}", p)); }
        if let Some(ref p) = self.t5xxl_path { parts.push(format!("t5:{}", p)); }
        if let Some(ref p) = self.llm_path { parts.push(format!("llm:{}", p)); }
        if let Some(ref p) = self.llm_vision_path { parts.push(format!("llmv:{}", p)); }
        if let Some(ref p) = self.diffusion_model_path { parts.push(format!("dm:{}", p)); }
        if let Some(ref p) = self.vae_path { parts.push(format!("vae:{}", p)); }
        if let Some(ref p) = self.control_net_path { parts.push(format!("cn:{}", p)); }
        if let Some(ref p) = self.text_encoder_path { parts.push(format!("te:{}", p)); }
        if let Some(ref p) = self.audio_vae_path { parts.push(format!("avae:{}", p)); }
        if let Some(ref p) = self.embeddings_connectors_path { parts.push(format!("ec:{}", p)); }
        if parts.is_empty() { "empty".to_string() } else { parts.join("|") }
    }
}

impl Default for ImageGenParams {
    fn default() -> Self {
        Self {
            model_config: ModelConfig::default(),
            loras: Vec::new(),
            prompt: String::new(),
            negative_prompt: String::new(),
            clip_skip: -1,
            init_image: None,
            ref_images: Vec::new(),
            auto_resize_ref_image: true,
            increase_ref_index: false,
            mask_image: None,
            width: 512,
            height: 512,
            sample_params: SampleParams::default(),
            strength: 0.75,
            seed: 42,
            batch_count: 1,
            control_image: None,
            control_strength: 0.9,
            vae_tiling_params: TilingParams::default(),
            cache_params: CacheParams::default(),
            hires_params: HiresParams::default(),
        }
    }
}

impl ImageGenParams {
    pub fn new(prompt: impl Into<String>) -> Self {
        Self {
            prompt: prompt.into(),
            ..Default::default()
        }
    }

    pub fn with_negative_prompt(mut self, prompt: impl Into<String>) -> Self {
        self.negative_prompt = prompt.into();
        self
    }

    pub fn with_dimensions(mut self, width: i32, height: i32) -> Self {
        self.width = width;
        self.height = height;
        self
    }

    pub fn with_seed(mut self, seed: i64) -> Self {
        self.seed = seed;
        self
    }

    pub fn with_sample_steps(mut self, steps: i32) -> Self {
        self.sample_params.sample_steps = steps;
        self
    }

    pub fn with_cfg_scale(mut self, cfg: f32) -> Self {
        self.sample_params.guidance.txt_cfg = cfg;
        self
    }

    pub fn with_sample_method(mut self, method: SampleMethod) -> Self {
        self.sample_params.sample_method = method;
        self
    }

    pub fn with_scheduler(mut self, scheduler: Scheduler) -> Self {
        self.sample_params.scheduler = scheduler;
        self
    }

    pub fn with_batch_count(mut self, count: i32) -> Self {
        self.batch_count = count;
        self
    }

    pub fn with_lora(mut self, path: impl Into<String>, multiplier: f32) -> Self {
        self.loras.push(LoraEntry {
            path: path.into(),
            multiplier,
            is_high_noise: false,
        });
        self
    }

    pub fn with_model_config(mut self, config: ModelConfig) -> Self {
        self.model_config = config;
        self
    }

    pub fn with_init_image(mut self, image: SdImage) -> Self {
        self.init_image = Some(image);
        self
    }

    pub fn with_mask_image(mut self, image: SdImage) -> Self {
        self.mask_image = Some(image);
        self
    }

    pub fn with_strength(mut self, strength: f32) -> Self {
        self.strength = strength;
        self
    }

    pub fn is_img2img(&self) -> bool {
        self.init_image.is_some()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoGenParams {
    pub model_config: ModelConfig,
    pub loras: Vec<LoraEntry>,
    pub prompt: String,
    pub negative_prompt: String,
    pub clip_skip: i32,
    pub init_image: Option<SdImage>,
    pub end_image: Option<SdImage>,
    pub control_frames: Vec<SdImage>,
    pub width: i32,
    pub height: i32,
    pub sample_params: SampleParams,
    pub high_noise_sample_params: Option<SampleParams>,
    pub moe_boundary: f32,
    pub strength: f32,
    pub seed: i64,
    pub video_frames: i32,
    pub vace_strength: f32,
    pub vae_tiling_params: TilingParams,
    pub cache_params: CacheParams,
    pub fps: i32,
}

impl Default for VideoGenParams {
    fn default() -> Self {
        Self {
            model_config: ModelConfig::default(),
            loras: Vec::new(),
            prompt: String::new(),
            negative_prompt: String::new(),
            clip_skip: -1,
            init_image: None,
            end_image: None,
            control_frames: Vec::new(),
            width: 512,
            height: 512,
            sample_params: SampleParams::default(),
            high_noise_sample_params: None,
            moe_boundary: 0.875,
            strength: 0.75,
            seed: 42,
            video_frames: 1,
            vace_strength: 1.0,
            vae_tiling_params: TilingParams::default(),
            cache_params: CacheParams::default(),
            fps: 24,
        }
    }
}

impl VideoGenParams {
    pub fn new(prompt: impl Into<String>) -> Self {
        Self {
            prompt: prompt.into(),
            ..Default::default()
        }
    }

    pub fn with_dimensions(mut self, width: i32, height: i32) -> Self {
        self.width = width;
        self.height = height;
        self
    }

    pub fn with_seed(mut self, seed: i64) -> Self {
        self.seed = seed;
        self
    }

    pub fn with_video_frames(mut self, frames: i32) -> Self {
        self.video_frames = frames;
        self
    }

    pub fn with_sample_steps(mut self, steps: i32) -> Self {
        self.sample_params.sample_steps = steps;
        self
    }

    pub fn with_cfg_scale(mut self, cfg: f32) -> Self {
        self.sample_params.guidance.txt_cfg = cfg;
        self
    }

    pub fn with_init_image(mut self, image: SdImage) -> Self {
        self.init_image = Some(image);
        self
    }

    pub fn with_end_image(mut self, image: SdImage) -> Self {
        self.end_image = Some(image);
        self
    }

    pub fn with_negative_prompt(mut self, prompt: impl Into<String>) -> Self {
        self.negative_prompt = prompt.into();
        self
    }

    pub fn with_model_config(mut self, config: ModelConfig) -> Self {
        self.model_config = config;
        self
    }

    pub fn with_fps(mut self, fps: i32) -> Self {
        self.fps = fps;
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpscaleParams {
    pub esrgan_path: String,
    pub offload_to_cpu: bool,
    pub direct: bool,
    pub n_threads: i32,
    pub tile_size: i32,
    pub upscale_factor: u32,
}

impl UpscaleParams {
    pub fn new(esrgan_path: impl Into<String>) -> Self {
        Self {
            esrgan_path: esrgan_path.into(),
            offload_to_cpu: false,
            direct: false,
            n_threads: -1,
            tile_size: 128,
            upscale_factor: 4,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConvertParams {
    pub input_path: String,
    pub vae_path: Option<String>,
    pub output_path: String,
    pub output_type: SdType,
    pub tensor_type_rules: Option<String>,
    pub convert_name: bool,
}

impl ConvertParams {
    pub fn new(input_path: impl Into<String>, output_path: impl Into<String>) -> Self {
        Self {
            input_path: input_path.into(),
            vae_path: None,
            output_path: output_path.into(),
            output_type: SdType::Q8_0,
            tensor_type_rules: None,
            convert_name: false,
        }
    }

    pub fn with_vae(mut self, path: impl Into<String>) -> Self {
        self.vae_path = Some(path.into());
        self
    }

    pub fn with_output_type(mut self, sd_type: SdType) -> Self {
        self.output_type = sd_type;
        self
    }

    pub fn with_tensor_type_rules(mut self, rules: impl Into<String>) -> Self {
        self.tensor_type_rules = Some(rules.into());
        self
    }

    pub fn with_convert_name(mut self, enable: bool) -> Self {
        self.convert_name = enable;
        self
    }
}

// ========== H3 (MiniMax-HunyuanVideoAudio) 相关参数 ==========

/// H3 生成模式
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum H3Mode {
    /// 文生视频+音频
    T2VA,
    /// 参考视频生视频+音频 (Ref2VA / Video-Ref)
    Ref2VA,
    /// 图生视频+音频 (I2VA / Image-Ref)
    I2VA,
    /// 多参考混合生视频+音频 (MR2VA)
    MR2VA,
    /// 音效生成
    SFX,
    /// 仅音频生成
    Audio,
}

impl Default for H3Mode {
    fn default() -> Self {
        H3Mode::T2VA
    }
}

/// H3 上下文信息（从 Context-IR 解析得到）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct H3Context {
    /// 主体描述
    pub subject: String,
    /// 环境/背景描述
    pub environment: String,
    /// 风格描述
    pub style: String,
    /// 镜头运动
    pub camera_motion: String,
    /// 音效描述列表
    pub sound_effects: Vec<String>,
    /// 背景音乐描述（可选）
    pub bgm: Option<String>,
    /// 原始解析出的负面提示词
    pub negative_prompt: Option<String>,
}

impl Default for H3Context {
    fn default() -> Self {
        Self {
            subject: String::new(),
            environment: String::new(),
            style: "cinematic, high quality".to_string(),
            camera_motion: String::new(),
            sound_effects: Vec::new(),
            bgm: None,
            negative_prompt: None,
        }
    }
}

impl H3Context {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_subject(mut self, subject: impl Into<String>) -> Self {
        self.subject = subject.into();
        self
    }

    pub fn with_environment(mut self, env: impl Into<String>) -> Self {
        self.environment = env.into();
        self
    }

    pub fn with_style(mut self, style: impl Into<String>) -> Self {
        self.style = style.into();
        self
    }

    pub fn with_camera_motion(mut self, motion: impl Into<String>) -> Self {
        self.camera_motion = motion.into();
        self
    }

    pub fn add_sound_effect(mut self, sfx: impl Into<String>) -> Self {
        self.sound_effects.push(sfx.into());
        self
    }

    pub fn with_bgm(mut self, bgm: impl Into<String>) -> Self {
        self.bgm = Some(bgm.into());
        self
    }

    /// 组合成正向提示词
    pub fn build_positive_prompt(&self) -> String {
        let mut parts = Vec::new();
        if !self.subject.is_empty() {
            parts.push(self.subject.clone());
        }
        if !self.environment.is_empty() {
            parts.push(self.environment.clone());
        }
        if !self.camera_motion.is_empty() {
            parts.push(self.camera_motion.clone());
        }
        if !self.sound_effects.is_empty() {
            parts.push(format!("sound effects: {}", self.sound_effects.join(", ")));
        }
        if let Some(ref bgm) = self.bgm {
            parts.push(format!("background music: {}", bgm));
        }
        if !self.style.is_empty() {
            parts.push(self.style.clone());
        }
        parts.join(", ")
    }
}

/// H3 (T2VA/Ref2VA/I2VA) 生成参数
#[derive(Debug, Clone)]
pub struct H3Params {
    /// 生成模式
    pub mode: H3Mode,
    /// 正向提示词
    pub prompt: String,
    /// 负向提示词
    pub negative_prompt: String,
    /// 推理步数
    pub num_inference_steps: i32,
    /// 引导系数 (CFG scale)
    pub guidance_scale: f64,
    /// 视频分辨率（宽）
    pub width: i32,
    /// 视频分辨率（高）
    pub height: i32,
    /// 视频总帧数 (H3约束: 17*n+5, 120-360)
    pub num_frames: i32,
    /// 视频帧率
    pub fps: i32,
    /// 音频持续时间（秒），默认匹配视频时长
    pub audio_duration: Option<f64>,
    /// 种子
    pub seed: i64,
    /// 上下文信息（从 Context-IR 解析）
    pub context: Option<H3Context>,
    /// 参考图像（I2VA/MR2VA）
    pub reference_images: Vec<crate::image::SdImage>,
    /// 参考视频（Ref2VA/MR2VA）
    pub reference_video: Option<crate::image::SdVideo>,
    /// 音频引导（可选）
    pub audio_guide: Option<crate::image::SdAudio>,
    /// 流匹配 shift 参数
    pub shift: Option<f64>,
    /// 是否自动生成音效
    pub generate_sfx: bool,
    /// 是否生成背景音乐
    pub generate_bgm: bool,
}

impl Default for H3Params {
    fn default() -> Self {
        Self {
            mode: H3Mode::T2VA,
            prompt: String::new(),
            negative_prompt: "low quality, blurry, distorted, static, noise".to_string(),
            num_inference_steps: 50,
            guidance_scale: 7.0,
            width: 848,
            height: 480,
            num_frames: 123, // 17*7+4=123, default ~4s
            fps: 24,
            audio_duration: None,
            seed: 42,
            context: None,
            reference_images: Vec::new(),
            reference_video: None,
            audio_guide: None,
            shift: None,
            generate_sfx: true,
            generate_bgm: false,
        }
    }
}

impl H3Params {
    pub fn new(prompt: impl Into<String>) -> Self {
        Self {
            prompt: prompt.into(),
            ..Default::default()
        }
    }

    pub fn t2va(prompt: impl Into<String>) -> Self {
        Self::new(prompt)
    }

    pub fn i2va(prompt: impl Into<String>, reference_image: crate::image::SdImage) -> Self {
        Self {
            mode: H3Mode::I2VA,
            prompt: prompt.into(),
            reference_images: vec![reference_image],
            ..Default::default()
        }
    }

    pub fn ref2va(prompt: impl Into<String>, reference_video: crate::image::SdVideo) -> Self {
        Self {
            mode: H3Mode::Ref2VA,
            prompt: prompt.into(),
            reference_video: Some(reference_video),
            ..Default::default()
        }
    }

    pub fn mr2va(prompt: impl Into<String>, images: Vec<crate::image::SdImage>) -> Self {
        Self {
            mode: H3Mode::MR2VA,
            prompt: prompt.into(),
            reference_images: images,
            ..Default::default()
        }
    }

    pub fn with_negative_prompt(mut self, neg: impl Into<String>) -> Self {
        self.negative_prompt = neg.into();
        self
    }

    pub fn with_steps(mut self, steps: i32) -> Self {
        self.num_inference_steps = steps;
        self
    }

    pub fn with_cfg(mut self, cfg: f64) -> Self {
        self.guidance_scale = cfg;
        self
    }

    pub fn with_resolution(mut self, width: i32, height: i32) -> Self {
        self.width = width;
        self.height = height;
        self
    }

    /// 设置帧数（自动对齐到 H3 约束: 17*n+5, 范围 120-360）
    pub fn with_num_frames(mut self, frames: i32) -> Self {
        // 对齐到 17*n+5
        let aligned = if frames < 120 { 123 }
        else if frames > 360 { 362 }
        else {
            let n = (frames as f64 / 17.0).round() as i32;
            let candidate = 17 * n + 5;
            candidate.max(123).min(362)
        };
        self.num_frames = aligned;
        self
    }

    pub fn with_fps(mut self, fps: i32) -> Self {
        self.fps = fps;
        self
    }

    pub fn with_seed(mut self, seed: i64) -> Self {
        self.seed = seed;
        self
    }

    pub fn with_context(mut self, ctx: H3Context) -> Self {
        self.context = Some(ctx);
        self
    }

    pub fn with_shift(mut self, shift: f64) -> Self {
        self.shift = Some(shift);
        self
    }

    pub fn with_sfx(mut self, enable: bool) -> Self {
        self.generate_sfx = enable;
        self
    }

    pub fn with_bgm(mut self, enable: bool) -> Self {
        self.generate_bgm = enable;
        self
    }

    pub fn with_audio_duration(mut self, duration: f64) -> Self {
        self.audio_duration = Some(duration);
        self
    }

    /// 获取视频时长（秒）
    pub fn video_duration_sec(&self) -> f64 {
        self.num_frames as f64 / self.fps as f64
    }

    /// 获取音频时长（秒），默认匹配视频
    pub fn get_audio_duration(&self) -> f64 {
        self.audio_duration.unwrap_or_else(|| self.video_duration_sec())
    }
}

/// Context-IR 参数
#[derive(Debug, Clone)]
pub struct ContextIrParams {
    /// 输入图像（可选，和video二选一）
    pub image: Option<crate::image::SdImage>,
    /// 输入视频（可选，和image二选一）
    pub video: Option<crate::image::SdVideo>,
    /// 用户附加文本提示（可选，补充说明）
    pub user_prompt: Option<String>,
    /// 是否解析音效
    pub parse_sfx: bool,
    /// 是否解析背景音乐
    pub parse_bgm: bool,
}

impl ContextIrParams {
    pub fn from_image(image: crate::image::SdImage) -> Self {
        Self {
            image: Some(image),
            video: None,
            user_prompt: None,
            parse_sfx: true,
            parse_bgm: false,
        }
    }

    pub fn from_video(video: crate::image::SdVideo) -> Self {
        Self {
            image: None,
            video: Some(video),
            user_prompt: None,
            parse_sfx: true,
            parse_bgm: false,
        }
    }

    pub fn from_text(text: impl Into<String>) -> Self {
        Self {
            image: None,
            video: None,
            user_prompt: Some(text.into()),
            parse_sfx: true,
            parse_bgm: false,
        }
    }

    pub fn with_user_prompt(mut self, prompt: impl Into<String>) -> Self {
        self.user_prompt = Some(prompt.into());
        self
    }

    pub fn with_sfx(mut self, enable: bool) -> Self {
        self.parse_sfx = enable;
        self
    }

    pub fn with_bgm(mut self, enable: bool) -> Self {
        self.parse_bgm = enable;
        self
    }
}

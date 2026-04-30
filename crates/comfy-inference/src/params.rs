use crate::image::SdImage;
use crate::types::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextConfig {
    pub model_path: Option<String>,
    pub clip_l_path: Option<String>,
    pub clip_g_path: Option<String>,
    pub clip_vision_path: Option<String>,
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

    pub fn with_mmap(mut self, enable: bool) -> Self {
        self.enable_mmap = enable;
        self
    }

    pub fn with_vae_decode_only(mut self, enable: bool) -> Self {
        self.vae_decode_only = enable;
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageGenParams {
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

impl Default for ImageGenParams {
    fn default() -> Self {
        Self {
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
}

impl Default for VideoGenParams {
    fn default() -> Self {
        Self {
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

#![allow(dead_code)]
#![allow(non_camel_case_types)]
#![allow(clippy::missing_safety_doc)]

use std::os::raw::{c_char, c_float, c_int, c_void};

pub type SdCtxT = c_void;
pub type UpscalerCtxT = c_void;

#[repr(u32)]
#[derive(Debug, Clone, Copy)]
pub enum CRngType {
    StdDefault = 0,
    Cuda = 1,
    Cpu = 2,
    Count = 3,
}

#[repr(u32)]
#[derive(Debug, Clone, Copy)]
pub enum CSampleMethod {
    Euler = 0,
    EulerA = 1,
    Heun = 2,
    DPM2 = 3,
    DPMPP2SA = 4,
    DPMPP2M = 5,
    DPMPP2Mv2 = 6,
    IPNDM = 7,
    IPNDMV = 8,
    LCM = 9,
    DDIMTrailing = 10,
    TCD = 11,
    ResMultistep = 12,
    Res2S = 13,
    ErSde = 14,
    Count = 15,
}

#[repr(u32)]
#[derive(Debug, Clone, Copy)]
pub enum CScheduler {
    Discrete = 0,
    Karras = 1,
    Exponential = 2,
    Ays = 3,
    Gits = 4,
    SgmUniform = 5,
    Simple = 6,
    Smoothstep = 7,
    KlOptimal = 8,
    Lcm = 9,
    BongTangent = 10,
    Count = 11,
}

#[repr(u32)]
#[derive(Debug, Clone, Copy)]
pub enum CPredictionType {
    Eps = 0,
    V = 1,
    EdmV = 2,
    Flow = 3,
    FluxFlow = 4,
    Flux2Flow = 5,
    Count = 6,
}

#[repr(u32)]
#[derive(Debug, Clone, Copy)]
pub enum CSdType {
    F32 = 0,
    F16 = 1,
    Q4_0 = 2,
    Q4_1 = 3,
    Q5_0 = 6,
    Q5_1 = 7,
    Q8_0 = 8,
    Q8_1 = 9,
    Q2_K = 10,
    Q3_K = 11,
    Q4_K = 12,
    Q5_K = 13,
    Q6_K = 14,
    Q8_K = 15,
    BF16 = 30,
    Count = 41,
}

#[repr(u32)]
#[derive(Debug, Clone, Copy)]
pub enum CLoraApplyMode {
    Auto = 0,
    Immediately = 1,
    AtRuntime = 2,
    Count = 3,
}

#[repr(u32)]
#[derive(Debug, Clone, Copy)]
pub enum CCacheMode {
    Disabled = 0,
    EasyCache = 1,
    UCache = 2,
    DBCache = 3,
    TaylorSeer = 4,
    CacheDit = 5,
    Spectrum = 6,
}

#[repr(u32)]
#[derive(Debug, Clone, Copy)]
pub enum CPreviewMode {
    None = 0,
    Proj = 1,
    Tae = 2,
    Vae = 3,
    Count = 4,
}

#[repr(u32)]
#[derive(Debug, Clone, Copy)]
pub enum CHiresUpscaler {
    None = 0,
    Latent = 1,
    LatentNearest = 2,
    LatentNearestExact = 3,
    LatentAntialiased = 4,
    LatentBicubic = 5,
    LatentBicubicAntialiased = 6,
    Lanczos = 7,
    Nearest = 8,
    Model = 9,
    Count = 10,
}

#[repr(C)]
#[derive(Debug, Clone)]
pub struct CEmbedding {
    pub name: *const c_char,
    pub path: *const c_char,
}

#[repr(C)]
#[derive(Debug)]
pub struct CSdCtxParams {
    pub model_path: *const c_char,
    pub clip_l_path: *const c_char,
    pub clip_g_path: *const c_char,
    pub clip_vision_path: *const c_char,
    pub t5xxl_path: *const c_char,
    pub llm_path: *const c_char,
    pub llm_vision_path: *const c_char,
    pub diffusion_model_path: *const c_char,
    pub high_noise_diffusion_model_path: *const c_char,
    pub vae_path: *const c_char,
    pub taesd_path: *const c_char,
    pub control_net_path: *const c_char,
    pub embeddings: *const CEmbedding,
    pub embedding_count: u32,
    pub photo_maker_path: *const c_char,
    pub tensor_type_rules: *const c_char,
    pub vae_decode_only: bool,
    pub free_params_immediately: bool,
    pub n_threads: c_int,
    pub wtype: CSdType,
    pub rng_type: CRngType,
    pub sampler_rng_type: CRngType,
    pub prediction: CPredictionType,
    pub lora_apply_mode: CLoraApplyMode,
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
    pub force_sdxl_vae_conv_scale: bool,
    pub chroma_use_dit_mask: bool,
    pub chroma_use_t5_mask: bool,
    pub chroma_t5_mask_pad: c_int,
    pub qwen_image_zero_cond_t: bool,
}

#[repr(C)]
#[derive(Debug, Clone)]
pub struct CSdImage {
    pub width: u32,
    pub height: u32,
    pub channel: u32,
    pub data: *mut u8,
}

#[repr(C)]
#[derive(Debug)]
pub struct CSlgParams {
    pub layers: *mut c_int,
    pub layer_count: usize,
    pub layer_start: c_float,
    pub layer_end: c_float,
    pub scale: c_float,
}

#[repr(C)]
#[derive(Debug)]
pub struct CGuidanceParams {
    pub txt_cfg: c_float,
    pub img_cfg: c_float,
    pub distilled_guidance: c_float,
    pub slg: CSlgParams,
}

#[repr(C)]
#[derive(Debug)]
pub struct CSampleParams {
    pub guidance: CGuidanceParams,
    pub scheduler: CScheduler,
    pub sample_method: CSampleMethod,
    pub sample_steps: c_int,
    pub eta: c_float,
    pub shifted_timestep: c_int,
    pub custom_sigmas: *mut c_float,
    pub custom_sigmas_count: c_int,
    pub flow_shift: c_float,
}

#[repr(C)]
#[derive(Debug)]
pub struct CPmParams {
    pub id_images: *mut CSdImage,
    pub id_images_count: c_int,
    pub id_embed_path: *const c_char,
    pub style_strength: c_float,
}

#[repr(C)]
#[derive(Debug)]
pub struct CTilingParams {
    pub enabled: bool,
    pub tile_size_x: c_int,
    pub tile_size_y: c_int,
    pub target_overlap: c_float,
    pub rel_size_x: c_float,
    pub rel_size_y: c_float,
}

#[repr(C)]
#[derive(Debug)]
pub struct CCacheParams {
    pub mode: CCacheMode,
    pub reuse_threshold: c_float,
    pub start_percent: c_float,
    pub end_percent: c_float,
    pub error_decay_rate: c_float,
    pub use_relative_threshold: bool,
    pub reset_error_on_compute: bool,
    pub fn_compute_blocks: c_int,
    pub bn_compute_blocks: c_int,
    pub residual_diff_threshold: c_float,
    pub max_warmup_steps: c_int,
    pub max_cached_steps: c_int,
    pub max_continuous_cached_steps: c_int,
    pub taylorseer_n_derivatives: c_int,
    pub taylorseer_skip_interval: c_int,
    pub scm_mask: *const c_char,
    pub scm_policy_dynamic: bool,
    pub spectrum_w: c_float,
    pub spectrum_m: c_int,
    pub spectrum_lam: c_float,
    pub spectrum_window_size: c_int,
    pub spectrum_flex_window: c_float,
    pub spectrum_warmup_steps: c_int,
    pub spectrum_stop_percent: c_float,
}

#[repr(C)]
#[derive(Debug)]
pub struct CLora {
    pub is_high_noise: bool,
    pub multiplier: c_float,
    pub path: *const c_char,
}

#[repr(C)]
#[derive(Debug)]
pub struct CHiresParams {
    pub enabled: bool,
    pub upscaler: CHiresUpscaler,
    pub model_path: *const c_char,
    pub scale: c_float,
    pub target_width: c_int,
    pub target_height: c_int,
    pub steps: c_int,
    pub denoising_strength: c_float,
    pub upscale_tile_size: c_int,
}

#[repr(C)]
#[derive(Debug)]
pub struct CImgGenParams {
    pub loras: *const CLora,
    pub lora_count: u32,
    pub prompt: *const c_char,
    pub negative_prompt: *const c_char,
    pub clip_skip: c_int,
    pub init_image: CSdImage,
    pub ref_images: *mut CSdImage,
    pub ref_images_count: c_int,
    pub auto_resize_ref_image: bool,
    pub increase_ref_index: bool,
    pub mask_image: CSdImage,
    pub width: c_int,
    pub height: c_int,
    pub sample_params: CSampleParams,
    pub strength: c_float,
    pub seed: i64,
    pub batch_count: c_int,
    pub control_image: CSdImage,
    pub control_strength: c_float,
    pub pm_params: CPmParams,
    pub vae_tiling_params: CTilingParams,
    pub cache: CCacheParams,
    pub hires: CHiresParams,
}

#[repr(C)]
#[derive(Debug)]
pub struct CVidGenParams {
    pub loras: *const CLora,
    pub lora_count: u32,
    pub prompt: *const c_char,
    pub negative_prompt: *const c_char,
    pub clip_skip: c_int,
    pub init_image: CSdImage,
    pub end_image: CSdImage,
    pub control_frames: *mut CSdImage,
    pub control_frames_size: c_int,
    pub width: c_int,
    pub height: c_int,
    pub sample_params: CSampleParams,
    pub high_noise_sample_params: CSampleParams,
    pub moe_boundary: c_float,
    pub strength: c_float,
    pub seed: i64,
    pub video_frames: c_int,
    pub vace_strength: c_float,
    pub vae_tiling_params: CTilingParams,
    pub cache: CCacheParams,
}

pub type SdLogCb = Option<unsafe extern "C" fn(level: u32, text: *const c_char, data: *mut c_void)>;
pub type SdProgressCb = Option<unsafe extern "C" fn(step: c_int, steps: c_int, time: c_float, data: *mut c_void)>;
pub type SdPreviewCb = Option<unsafe extern "C" fn(step: c_int, frame_count: c_int, frames: *mut CSdImage, is_noisy: bool, data: *mut c_void)>;

#[cfg(feature = "local")]
#[link(name = "stable-diffusion")]
extern "C" {
    pub fn sd_set_log_callback(cb: SdLogCb, data: *mut c_void);
    pub fn sd_set_progress_callback(cb: SdProgressCb, data: *mut c_void);
    pub fn sd_set_preview_callback(cb: SdPreviewCb, mode: CPreviewMode, interval: c_int, denoised: bool, noisy: bool, data: *mut c_void);
    pub fn sd_get_num_physical_cores() -> c_int;
    pub fn sd_get_system_info() -> *const c_char;
    pub fn sd_ctx_supports_image_generation(ctx: *const SdCtxT) -> bool;
    pub fn sd_ctx_supports_video_generation(ctx: *const SdCtxT) -> bool;

    pub fn sd_type_name(sd_type: CSdType) -> *const c_char;
    pub fn str_to_sd_type(str: *const c_char) -> CSdType;
    pub fn sd_rng_type_name(rng_type: CRngType) -> *const c_char;
    pub fn str_to_rng_type(str: *const c_char) -> CRngType;
    pub fn sd_sample_method_name(method: CSampleMethod) -> *const c_char;
    pub fn str_to_sample_method(str: *const c_char) -> CSampleMethod;
    pub fn sd_scheduler_name(scheduler: CScheduler) -> *const c_char;
    pub fn str_to_scheduler(str: *const c_char) -> CScheduler;
    pub fn sd_prediction_name(prediction: CPredictionType) -> *const c_char;
    pub fn str_to_prediction(str: *const c_char) -> CPredictionType;

    pub fn sd_ctx_params_init(params: *mut CSdCtxParams);
    pub fn new_sd_ctx(params: *const CSdCtxParams) -> *mut SdCtxT;
    pub fn free_sd_ctx(ctx: *mut SdCtxT);

    pub fn sd_sample_params_init(params: *mut CSampleParams);
    pub fn sd_get_default_sample_method(ctx: *const SdCtxT) -> CSampleMethod;
    pub fn sd_get_default_scheduler(ctx: *const SdCtxT, method: CSampleMethod) -> CScheduler;

    pub fn sd_img_gen_params_init(params: *mut CImgGenParams);
    pub fn generate_image(ctx: *mut SdCtxT, params: *const CImgGenParams) -> *mut CSdImage;

    pub fn sd_vid_gen_params_init(params: *mut CVidGenParams);
    pub fn generate_video(ctx: *mut SdCtxT, params: *const CVidGenParams, num_frames_out: *mut c_int) -> *mut CSdImage;

    pub fn new_upscaler_ctx(
        esrgan_path: *const c_char,
        offload_to_cpu: bool,
        direct: bool,
        n_threads: c_int,
        tile_size: c_int,
    ) -> *mut UpscalerCtxT;
    pub fn free_upscaler_ctx(ctx: *mut UpscalerCtxT);
    pub fn upscale(ctx: *mut UpscalerCtxT, input: CSdImage, upscale_factor: u32) -> CSdImage;
    pub fn get_upscale_factor(ctx: *mut UpscalerCtxT) -> c_int;

    pub fn convert(
        input_path: *const c_char,
        vae_path: *const c_char,
        output_path: *const c_char,
        output_type: CSdType,
        tensor_type_rules: *const c_char,
        convert_name: bool,
    ) -> bool;

    pub fn sd_commit() -> *const c_char;
    pub fn sd_version() -> *const c_char;
}

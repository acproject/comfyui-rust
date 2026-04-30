use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u32)]
pub enum SdType {
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
    IQ2_XXS = 16,
    IQ2_XS = 17,
    IQ3_XXS = 18,
    IQ1_S = 19,
    IQ4_NL = 20,
    IQ3_S = 21,
    IQ2_S = 22,
    IQ4_XS = 23,
    I8 = 24,
    I16 = 25,
    I32 = 26,
    I64 = 27,
    F64 = 28,
    IQ1_M = 29,
    BF16 = 30,
    TQ1_0 = 34,
    TQ2_0 = 35,
    MXFP4 = 39,
    NVFP4 = 40,
    Auto = 41,
}

impl SdType {
    pub fn from_c(val: u32) -> Option<Self> {
        match val {
            0 => Some(SdType::F32),
            1 => Some(SdType::F16),
            2 => Some(SdType::Q4_0),
            3 => Some(SdType::Q4_1),
            6 => Some(SdType::Q5_0),
            7 => Some(SdType::Q5_1),
            8 => Some(SdType::Q8_0),
            9 => Some(SdType::Q8_1),
            10 => Some(SdType::Q2_K),
            11 => Some(SdType::Q3_K),
            12 => Some(SdType::Q4_K),
            13 => Some(SdType::Q5_K),
            14 => Some(SdType::Q6_K),
            15 => Some(SdType::Q8_K),
            30 => Some(SdType::BF16),
            41 => Some(SdType::Auto),
            _ => None,
        }
    }

    pub fn to_c(self) -> u32 {
        self as u32
    }
}

impl fmt::Display for SdType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            SdType::F32 => "f32",
            SdType::F16 => "f16",
            SdType::BF16 => "bf16",
            SdType::Q4_0 => "q4_0",
            SdType::Q4_1 => "q4_1",
            SdType::Q5_0 => "q5_0",
            SdType::Q5_1 => "q5_1",
            SdType::Q8_0 => "q8_0",
            SdType::Q8_1 => "q8_1",
            SdType::Q2_K => "q2_k",
            SdType::Q3_K => "q3_k",
            SdType::Q4_K => "q4_k",
            SdType::Q5_K => "q5_k",
            SdType::Q6_K => "q6_k",
            SdType::Q8_K => "q8_k",
            SdType::Auto => "auto",
            other => return write!(f, "sd_type_{}", *other as u32),
        };
        write!(f, "{}", s)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u32)]
pub enum RngType {
    StdDefault = 0,
    Cuda = 1,
    Cpu = 2,
}

impl RngType {
    pub fn from_c(val: u32) -> Option<Self> {
        match val {
            0 => Some(RngType::StdDefault),
            1 => Some(RngType::Cuda),
            2 => Some(RngType::Cpu),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u32)]
pub enum SampleMethod {
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
}

impl SampleMethod {
    pub fn from_c(val: u32) -> Option<Self> {
        match val {
            0 => Some(SampleMethod::Euler),
            1 => Some(SampleMethod::EulerA),
            2 => Some(SampleMethod::Heun),
            3 => Some(SampleMethod::DPM2),
            4 => Some(SampleMethod::DPMPP2SA),
            5 => Some(SampleMethod::DPMPP2M),
            6 => Some(SampleMethod::DPMPP2Mv2),
            7 => Some(SampleMethod::IPNDM),
            8 => Some(SampleMethod::IPNDMV),
            9 => Some(SampleMethod::LCM),
            10 => Some(SampleMethod::DDIMTrailing),
            11 => Some(SampleMethod::TCD),
            12 => Some(SampleMethod::ResMultistep),
            13 => Some(SampleMethod::Res2S),
            14 => Some(SampleMethod::ErSde),
            _ => None,
        }
    }

    pub fn to_c(self) -> u32 {
        self as u32
    }
}

impl fmt::Display for SampleMethod {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            SampleMethod::Euler => "euler",
            SampleMethod::EulerA => "euler_a",
            SampleMethod::Heun => "heun",
            SampleMethod::DPM2 => "dpm2",
            SampleMethod::DPMPP2SA => "dpmpp_2s_a",
            SampleMethod::DPMPP2M => "dpmpp_2m",
            SampleMethod::DPMPP2Mv2 => "dpmpp_2m_v2",
            SampleMethod::IPNDM => "ipndm",
            SampleMethod::IPNDMV => "ipndm_v",
            SampleMethod::LCM => "lcm",
            SampleMethod::DDIMTrailing => "ddim_trailing",
            SampleMethod::TCD => "tcd",
            SampleMethod::ResMultistep => "res_multistep",
            SampleMethod::Res2S => "res_2s",
            SampleMethod::ErSde => "er_sde",
        };
        write!(f, "{}", s)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u32)]
pub enum Scheduler {
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
}

impl Scheduler {
    pub fn from_c(val: u32) -> Option<Self> {
        match val {
            0 => Some(Scheduler::Discrete),
            1 => Some(Scheduler::Karras),
            2 => Some(Scheduler::Exponential),
            3 => Some(Scheduler::Ays),
            4 => Some(Scheduler::Gits),
            5 => Some(Scheduler::SgmUniform),
            6 => Some(Scheduler::Simple),
            7 => Some(Scheduler::Smoothstep),
            8 => Some(Scheduler::KlOptimal),
            9 => Some(Scheduler::Lcm),
            10 => Some(Scheduler::BongTangent),
            _ => None,
        }
    }

    pub fn to_c(self) -> u32 {
        self as u32
    }
}

impl fmt::Display for Scheduler {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Scheduler::Discrete => "discrete",
            Scheduler::Karras => "karras",
            Scheduler::Exponential => "exponential",
            Scheduler::Ays => "ays",
            Scheduler::Gits => "gits",
            Scheduler::SgmUniform => "sgm_uniform",
            Scheduler::Simple => "simple",
            Scheduler::Smoothstep => "smoothstep",
            Scheduler::KlOptimal => "kl_optimal",
            Scheduler::Lcm => "lcm",
            Scheduler::BongTangent => "bong_tangent",
        };
        write!(f, "{}", s)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u32)]
pub enum PredictionType {
    Eps = 0,
    V = 1,
    EdmV = 2,
    Flow = 3,
    FluxFlow = 4,
    Flux2Flow = 5,
}

impl PredictionType {
    pub fn from_c(val: u32) -> Option<Self> {
        match val {
            0 => Some(PredictionType::Eps),
            1 => Some(PredictionType::V),
            2 => Some(PredictionType::EdmV),
            3 => Some(PredictionType::Flow),
            4 => Some(PredictionType::FluxFlow),
            5 => Some(PredictionType::Flux2Flow),
            _ => None,
        }
    }

    pub fn to_c(self) -> u32 {
        self as u32
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u32)]
pub enum LoraApplyMode {
    Auto = 0,
    Immediately = 1,
    AtRuntime = 2,
}

impl LoraApplyMode {
    pub fn from_c(val: u32) -> Option<Self> {
        match val {
            0 => Some(LoraApplyMode::Auto),
            1 => Some(LoraApplyMode::Immediately),
            2 => Some(LoraApplyMode::AtRuntime),
            _ => None,
        }
    }

    pub fn to_c(self) -> u32 {
        self as u32
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u32)]
pub enum CacheMode {
    Disabled = 0,
    EasyCache = 1,
    UCache = 2,
    DBCache = 3,
    TaylorSeer = 4,
    CacheDit = 5,
    Spectrum = 6,
}

impl CacheMode {
    pub fn from_c(val: u32) -> Option<Self> {
        match val {
            0 => Some(CacheMode::Disabled),
            1 => Some(CacheMode::EasyCache),
            2 => Some(CacheMode::UCache),
            3 => Some(CacheMode::DBCache),
            4 => Some(CacheMode::TaylorSeer),
            5 => Some(CacheMode::CacheDit),
            6 => Some(CacheMode::Spectrum),
            _ => None,
        }
    }

    pub fn to_c(self) -> u32 {
        self as u32
    }
}

impl fmt::Display for CacheMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            CacheMode::Disabled => "disabled",
            CacheMode::EasyCache => "easycache",
            CacheMode::UCache => "ucache",
            CacheMode::DBCache => "dbcache",
            CacheMode::TaylorSeer => "taylorseer",
            CacheMode::CacheDit => "cache-dit",
            CacheMode::Spectrum => "spectrum",
        };
        write!(f, "{}", s)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u32)]
pub enum PreviewMode {
    None = 0,
    Proj = 1,
    Tae = 2,
    Vae = 3,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u32)]
pub enum HiresUpscaler {
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
}

impl fmt::Display for HiresUpscaler {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            HiresUpscaler::None => "None",
            HiresUpscaler::Latent => "Latent",
            HiresUpscaler::LatentNearest => "Latent (nearest)",
            HiresUpscaler::LatentNearestExact => "Latent (nearest-exact)",
            HiresUpscaler::LatentAntialiased => "Latent (antialiased)",
            HiresUpscaler::LatentBicubic => "Latent (bicubic)",
            HiresUpscaler::LatentBicubicAntialiased => "Latent (bicubic antialiased)",
            HiresUpscaler::Lanczos => "Lanczos",
            HiresUpscaler::Nearest => "Nearest",
            HiresUpscaler::Model => "Model",
        };
        write!(f, "{}", s)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LogLevel {
    Debug,
    Info,
    Warn,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TilingParams {
    pub enabled: bool,
    pub tile_size_x: i32,
    pub tile_size_y: i32,
    pub target_overlap: f32,
    pub rel_size_x: f32,
    pub rel_size_y: f32,
}

impl Default for TilingParams {
    fn default() -> Self {
        Self {
            enabled: false,
            tile_size_x: 0,
            tile_size_y: 0,
            target_overlap: 0.5,
            rel_size_x: 0.0,
            rel_size_y: 0.0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlgParams {
    pub layers: Vec<i32>,
    pub layer_start: f32,
    pub layer_end: f32,
    pub scale: f32,
}

impl Default for SlgParams {
    fn default() -> Self {
        Self {
            layers: vec![7, 8, 9],
            layer_start: 0.0,
            layer_end: 1.0,
            scale: 0.0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GuidanceParams {
    pub txt_cfg: f32,
    pub img_cfg: Option<f32>,
    pub distilled_guidance: f32,
    pub slg: SlgParams,
}

impl Default for GuidanceParams {
    fn default() -> Self {
        Self {
            txt_cfg: 7.0,
            img_cfg: None,
            distilled_guidance: 3.5,
            slg: SlgParams::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SampleParams {
    pub guidance: GuidanceParams,
    pub scheduler: Scheduler,
    pub sample_method: SampleMethod,
    pub sample_steps: i32,
    pub eta: Option<f32>,
    pub shifted_timestep: i32,
    pub flow_shift: Option<f32>,
}

impl Default for SampleParams {
    fn default() -> Self {
        Self {
            guidance: GuidanceParams::default(),
            scheduler: Scheduler::Discrete,
            sample_method: SampleMethod::EulerA,
            sample_steps: 20,
            eta: None,
            shifted_timestep: 0,
            flow_shift: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheParams {
    pub mode: CacheMode,
    pub reuse_threshold: f32,
    pub start_percent: f32,
    pub end_percent: f32,
    pub error_decay_rate: f32,
    pub use_relative_threshold: bool,
    pub reset_error_on_compute: bool,
}

impl Default for CacheParams {
    fn default() -> Self {
        Self {
            mode: CacheMode::Disabled,
            reuse_threshold: 0.0,
            start_percent: 0.0,
            end_percent: 1.0,
            error_decay_rate: 0.0,
            use_relative_threshold: false,
            reset_error_on_compute: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoraEntry {
    pub path: String,
    pub multiplier: f32,
    pub is_high_noise: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HiresParams {
    pub enabled: bool,
    pub upscaler: HiresUpscaler,
    pub model_path: Option<String>,
    pub scale: f32,
    pub target_width: i32,
    pub target_height: i32,
    pub steps: i32,
    pub denoising_strength: f32,
    pub upscale_tile_size: i32,
}

impl Default for HiresParams {
    fn default() -> Self {
        Self {
            enabled: false,
            upscaler: HiresUpscaler::Latent,
            model_path: None,
            scale: 2.0,
            target_width: 0,
            target_height: 0,
            steps: 0,
            denoising_strength: 0.7,
            upscale_tile_size: 128,
        }
    }
}

use crate::backend::InferenceBackend;
use crate::error::{InferenceError, InferenceResult};
use crate::ffi::*;
use crate::image::{SdImage, SdVideo};
use crate::params::*;
use crate::types::*;
use std::collections::HashMap;
use std::ffi::{c_char, c_int, c_void, CString, NulError};
use std::path::PathBuf;
use std::ptr;
use std::sync::Mutex;

extern "C" {
    fn free(ptr: *mut c_void);
}

/// Derive the models base directory from a model path like "models/checkpoints/model.safetensors"
fn derive_models_base_dir(model_path: &str) -> Option<PathBuf> {
    let path = PathBuf::from(model_path);
    // Go up from "checkpoints/model.safetensors" to get the base "models" dir
    path.parent().and_then(|p| p.parent()).map(|p| p.to_path_buf())
}

/// Find a file in a directory whose name contains any of the given substrings
fn find_file_in_dir(dir: &PathBuf, substrings: &[&str]) -> Option<String> {
    if !dir.exists() {
        return None;
    }
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            let lower = name.to_lowercase();
            for sub in substrings {
                if lower.contains(sub) {
                    return Some(entry.path().to_string_lossy().to_string());
                }
            }
        }
    }
    None
}

pub struct LocalBackend {
    contexts: Mutex<ContextCache>,
    base_config: ContextConfig,
}

unsafe impl Send for LocalBackend {}
unsafe impl Sync for LocalBackend {}

struct ContextCache {
    entries: HashMap<String, *mut SdCtxT>,
}

impl ContextCache {
    fn new() -> Self {
        Self {
            entries: HashMap::new(),
        }
    }

    fn get_or_create(
        &mut self,
        model_config: &ModelConfig,
        base_config: &ContextConfig,
    ) -> InferenceResult<*mut SdCtxT> {
        let key = model_config.cache_key();

        if let Some(&ctx) = self.entries.get(&key) {
            if !ctx.is_null() {
                return Ok(ctx);
            }
        }

        let ctx_config = build_ctx_config(model_config, base_config);
        let ctx = create_sd_ctx(&ctx_config)?;

        self.entries.insert(key, ctx);
        Ok(ctx)
    }

    fn clear(&mut self) {
        for (_, ctx) in self.entries.drain() {
            if !ctx.is_null() {
                unsafe {
                    free_sd_ctx(ctx);
                }
            }
        }
    }
}

impl Drop for ContextCache {
    fn drop(&mut self) {
        self.clear();
    }
}

fn build_ctx_config(model_config: &ModelConfig, base_config: &ContextConfig) -> ContextConfig {
    let is_ltx_model = model_config.model_path.as_ref().map_or(false, |p| {
        let p = p.to_lowercase();
        p.contains("ltx")
    }) || model_config.diffusion_model_path.as_ref().map_or(false, |p| {
        let p = p.to_lowercase();
        p.contains("ltx")
    });

    let mut config = ContextConfig {
        model_path: model_config.model_path.clone().or(base_config.model_path.clone()),
        clip_l_path: model_config.clip_l_path.clone().or(base_config.clip_l_path.clone()),
        clip_g_path: model_config.clip_g_path.clone().or(base_config.clip_g_path.clone()),
        clip_vision_path: model_config.clip_vision_path.clone().or(base_config.clip_vision_path.clone()),
        decoder_path: model_config.decoder_path.clone().or(base_config.decoder_path.clone()),
        rmbg_path: model_config.rmbg_path.clone().or(base_config.rmbg_path.clone()),
        t5xxl_path: model_config.t5xxl_path.clone().or(base_config.t5xxl_path.clone()),
        llm_path: model_config.llm_path.clone().or(base_config.llm_path.clone()),
        llm_vision_path: model_config.llm_vision_path.clone().or(base_config.llm_vision_path.clone()),
        diffusion_model_path: model_config.diffusion_model_path.clone().or(base_config.diffusion_model_path.clone()),
        vae_path: model_config.vae_path.clone().or(base_config.vae_path.clone()),
        control_net_path: model_config.control_net_path.clone().or(base_config.control_net_path.clone()),
        text_encoder_path: model_config.text_encoder_path.clone().or(base_config.text_encoder_path.clone()),
        embeddings_connectors_path: base_config.embeddings_connectors_path.clone(),
        audio_vae_path: base_config.audio_vae_path.clone(),
        ..base_config.clone()
    };

    if is_ltx_model {
        config.offload_params_to_cpu = true;
        // Don't auto-set Q8_0 for LTX models - CUDA binbcast doesn't support quantized types.
        // The fp8 safetensors file is already mapped to bf16 internally.
        // if matches!(config.wtype, SdType::Auto) {
        //     config.wtype = SdType::Q8_0;
        // }
        // For LTX models, text_encoder_path should map to llm_path if llm_path is not set
        if config.llm_path.is_none() {
            if let Some(ref te_path) = config.text_encoder_path {
                config.llm_path = Some(te_path.clone());
            }
        }

        // Auto-detect embeddings_connectors_path for LTX-2.3 models
        if config.embeddings_connectors_path.is_none() {
            if let Some(ref model_path) = config.model_path {
                if let Some(base_dir) = derive_models_base_dir(model_path) {
                    let te_dir = base_dir.join("text_encoders");
                    if let Some(path) = find_file_in_dir(&te_dir, &["embeddings_connectors", "embedding_connector"]) {
                        config.embeddings_connectors_path = Some(path);
                        tracing::info!("LTX: auto-detected embeddings_connectors_path={:?}", config.embeddings_connectors_path);
                    }
                }
            }
        }

        // Auto-detect audio_vae_path for LTX-2.3 models
        if config.audio_vae_path.is_none() {
            if let Some(ref model_path) = config.model_path {
                if let Some(base_dir) = derive_models_base_dir(model_path) {
                    let vae_dir = base_dir.join("vae");
                    if let Some(path) = find_file_in_dir(&vae_dir, &["audio_vae", "ltx-2.3"]) {
                        config.audio_vae_path = Some(path);
                        tracing::info!("LTX: auto-detected audio_vae_path={:?}", config.audio_vae_path);
                    }
                }
            }
        }

        tracing::info!("LTX model detected: enabling offload_params_to_cpu, wtype={:?}, llm_path={:?}, embeddings_connectors_path={:?}, audio_vae_path={:?}",
            config.wtype, config.llm_path, config.embeddings_connectors_path, config.audio_vae_path);
    }

    config
}

fn create_sd_ctx(config: &ContextConfig) -> InferenceResult<*mut SdCtxT> {
    tracing::info!("Creating SD context with model_path={:?}, diffusion_model_path={:?}, clip_vision_path={:?}, vae_path={:?}",
        config.model_path, config.diffusion_model_path, config.clip_vision_path, config.vae_path);

    let mut c_params: CSdCtxParams = unsafe { std::mem::zeroed() };

    unsafe {
        sd_ctx_params_init(&mut c_params);
    }

    let mut strings = CStringHolder::new();
    c_params.model_path = strings.opt_cstr(&config.model_path);
    c_params.clip_l_path = strings.opt_cstr(&config.clip_l_path);
    c_params.clip_g_path = strings.opt_cstr(&config.clip_g_path);
    c_params.clip_vision_path = strings.opt_cstr(&config.clip_vision_path);
    c_params.t5xxl_path = strings.opt_cstr(&config.t5xxl_path);
    c_params.llm_path = strings.opt_cstr(&config.llm_path);
    c_params.llm_vision_path = strings.opt_cstr(&config.llm_vision_path);
    c_params.diffusion_model_path = strings.opt_cstr(&config.diffusion_model_path);
    c_params.high_noise_diffusion_model_path = strings.opt_cstr(&config.high_noise_diffusion_model_path);
    c_params.vae_path = strings.opt_cstr(&config.vae_path);
    c_params.taesd_path = strings.opt_cstr(&config.taesd_path);
    c_params.control_net_path = strings.opt_cstr(&config.control_net_path);
    c_params.photo_maker_path = strings.opt_cstr(&config.photo_maker_path);
    c_params.tensor_type_rules = strings.opt_cstr(&config.tensor_type_rules);
    c_params.embeddings_connectors_path = strings.opt_cstr(&config.embeddings_connectors_path);
    c_params.audio_vae_path = strings.opt_cstr(&config.audio_vae_path);

    c_params.vae_decode_only = config.vae_decode_only;
    c_params.free_params_immediately = config.free_params_immediately;
    c_params.n_threads = config.n_threads;
    c_params.wtype = match config.wtype {
        SdType::Auto => CSdType::Count,
        other => unsafe { std::mem::transmute::<u32, CSdType>(other as u32) },
    };
    c_params.rng_type = CRngType::Cuda;
    c_params.sampler_rng_type = CRngType::Count;
    c_params.prediction = CPredictionType::Count;
    c_params.lora_apply_mode = CLoraApplyMode::Auto;
    c_params.offload_params_to_cpu = config.offload_params_to_cpu;
    c_params.enable_mmap = config.enable_mmap;
    c_params.multi_gpu = config.multi_gpu;
    c_params.keep_clip_on_cpu = config.keep_clip_on_cpu;
    c_params.keep_control_net_on_cpu = config.keep_control_net_on_cpu;
    c_params.keep_vae_on_cpu = config.keep_vae_on_cpu;
    c_params.flash_attn = config.flash_attn;
    c_params.diffusion_flash_attn = config.diffusion_flash_attn;

    let ctx = unsafe { new_sd_ctx(&c_params) };
    if ctx.is_null() {
        tracing::error!("new_sd_ctx returned null - C++ context creation failed. Check stderr for C++ log output.");
        return Err(InferenceError::ContextCreationFailed);
    }

    Ok(ctx)
}

pub fn convert_model(params: ConvertParams) -> InferenceResult<bool> {
    let c_input = CString::new(params.input_path.as_str())
        .map_err(|e: NulError| InferenceError::InvalidParameter(e.to_string()))?;
    let c_output = CString::new(params.output_path.as_str())
        .map_err(|e: NulError| InferenceError::InvalidParameter(e.to_string()))?;
    let c_vae = params
        .vae_path
        .as_deref()
        .map(|s| CString::new(s).map_err(|e: NulError| InferenceError::InvalidParameter(e.to_string())))
        .transpose()?;
    let c_rules = params
        .tensor_type_rules
        .as_deref()
        .map(|s| CString::new(s).map_err(|e: NulError| InferenceError::InvalidParameter(e.to_string())))
        .transpose()?;

    let c_sd_type = match params.output_type {
        SdType::F32 => CSdType::F32,
        SdType::F16 => CSdType::F16,
        SdType::Q4_0 => CSdType::Q4_0,
        SdType::Q4_1 => CSdType::Q4_1,
        SdType::Q5_0 => CSdType::Q5_0,
        SdType::Q5_1 => CSdType::Q5_1,
        SdType::Q8_0 => CSdType::Q8_0,
        SdType::Q8_1 => CSdType::Q8_1,
        SdType::Q2_K => CSdType::Q2_K,
        SdType::Q3_K => CSdType::Q3_K,
        SdType::Q4_K => CSdType::Q4_K,
        SdType::Q5_K => CSdType::Q5_K,
        SdType::Q6_K => CSdType::Q6_K,
        SdType::Q8_K => CSdType::Q8_K,
        SdType::BF16 => CSdType::BF16,
        _ => CSdType::F16,
    };

    let result = unsafe {
        convert(
            c_input.as_ptr(),
            c_vae.as_ref().map_or(ptr::null(), |s| s.as_ptr()),
            c_output.as_ptr(),
            c_sd_type,
            c_rules.as_ref().map_or(ptr::null(), |s| s.as_ptr()),
            params.convert_name,
        )
    };

    Ok(result)
}

pub fn get_system_info() -> String {
    unsafe {
        let ptr = sd_get_system_info();
        if ptr.is_null() {
            return "unknown".to_string();
        }
        let c_str = std::ffi::CStr::from_ptr(ptr);
        c_str.to_string_lossy().to_string()
    }
}

pub fn get_version() -> String {
    unsafe {
        let ptr = sd_version();
        if ptr.is_null() {
            return "unknown".to_string();
        }
        let c_str = std::ffi::CStr::from_ptr(ptr);
        c_str.to_string_lossy().to_string()
    }
}

pub fn get_commit() -> String {
    unsafe {
        let ptr = sd_commit();
        if ptr.is_null() {
            return "unknown".to_string();
        }
        let c_str = std::ffi::CStr::from_ptr(ptr);
        c_str.to_string_lossy().to_string()
    }
}

pub fn get_num_physical_cores() -> i32 {
    unsafe { sd_get_num_physical_cores() }
}

/// C++ log callback - forwards C++ LOG_* output to Rust tracing
unsafe extern "C" fn sd_log_callback(level: u32, text: *const c_char, _data: *mut c_void) {
    if text.is_null() {
        return;
    }
    let msg = std::ffi::CStr::from_ptr(text).to_string_lossy();
    let trimmed = msg.trim();
    if trimmed.is_empty() {
        return;
    }
    // SD_LOG_DEBUG=0, SD_LOG_INFO=1, SD_LOG_WARN=2, SD_LOG_ERROR=3
    match level {
        3 => tracing::error!("[C++] {}", trimmed),
        2 => tracing::warn!("[C++] {}", trimmed),
        1 => tracing::info!("[C++] {}", trimmed),
        _ => tracing::debug!("[C++] {}", trimmed),
    }
}

impl LocalBackend {
    pub fn new(config: ContextConfig) -> InferenceResult<Self> {
        // Set up C++ log callback to forward to Rust tracing
        unsafe {
            sd_set_log_callback(Some(sd_log_callback), std::ptr::null_mut());
        }
        Ok(Self {
            contexts: Mutex::new(ContextCache::new()),
            base_config: config,
        })
    }

    pub fn base_config(&self) -> &ContextConfig {
        &self.base_config
    }

    pub fn load_model(&self, model_config: &ModelConfig) -> InferenceResult<()> {
        let mut cache = self.contexts.lock().unwrap();
        cache.get_or_create(model_config, &self.base_config)?;
        Ok(())
    }

    pub fn unload_model(&self, model_config: &ModelConfig) -> InferenceResult<()> {
        let mut cache = self.contexts.lock().unwrap();
        let key = model_config.cache_key();
        if let Some(ctx) = cache.entries.remove(&key) {
            if !ctx.is_null() {
                unsafe {
                    free_sd_ctx(ctx);
                }
            }
        }
        Ok(())
    }

    pub fn unload_all_models(&self) -> InferenceResult<()> {
        let mut cache = self.contexts.lock().unwrap();
        cache.clear();
        Ok(())
    }

    pub fn loaded_models(&self) -> Vec<String> {
        let cache = self.contexts.lock().unwrap();
        cache.entries.keys().cloned().collect()
    }

    fn build_c_img_gen_params(
        params: &ImageGenParams,
        strings: &mut CStringHolder,
    ) -> CImgGenParams {
        let mut c_params: CImgGenParams = unsafe { std::mem::zeroed() };
        unsafe { sd_img_gen_params_init(&mut c_params) };

        c_params.prompt = strings.cstr(&params.prompt);
        c_params.negative_prompt = strings.cstr(&params.negative_prompt);
        c_params.clip_skip = params.clip_skip;
        c_params.width = params.width;
        c_params.height = params.height;
        c_params.strength = params.strength;
        c_params.seed = params.seed;
        c_params.batch_count = params.batch_count;
        c_params.control_strength = params.control_strength;
        c_params.auto_resize_ref_image = params.auto_resize_ref_image;
        c_params.increase_ref_index = params.increase_ref_index;

        c_params.sample_params = build_c_sample_params(&params.sample_params);

        c_params.init_image = params
            .init_image
            .as_ref()
            .map(|img| image_to_c(img))
            .unwrap_or_else(null_c_image);

        c_params.mask_image = params
            .mask_image
            .as_ref()
            .map(|img| image_to_c(img))
            .unwrap_or_else(null_c_image);

        c_params.control_image = params
            .control_image
            .as_ref()
            .map(|img| image_to_c(img))
            .unwrap_or_else(null_c_image);

        c_params.vae_tiling_params = CTilingParams {
            enabled: params.vae_tiling_params.enabled,
            temporal_tiling: false,
            tile_size_x: params.vae_tiling_params.tile_size_x,
            tile_size_y: params.vae_tiling_params.tile_size_y,
            target_overlap: params.vae_tiling_params.target_overlap,
            rel_size_x: params.vae_tiling_params.rel_size_x,
            rel_size_y: params.vae_tiling_params.rel_size_y,
            extra_tiling_args: ptr::null(),
        };

        c_params
    }
}

impl InferenceBackend for LocalBackend {
    fn supports_image_generation(&self) -> bool {
        true
    }

    fn supports_video_generation(&self) -> bool {
        true
    }

    fn supports_3d_generation(&self) -> bool {
        false
    }

    fn generate_image(&self, params: ImageGenParams) -> InferenceResult<Vec<SdImage>> {
        let mut cache = self.contexts.lock().unwrap();
        let ctx = cache.get_or_create(&params.model_config, &self.base_config)?;

        let mut strings = CStringHolder::new();
        let c_params = Self::build_c_img_gen_params(&params, &mut strings);

        let result = unsafe { generate_image(ctx, &c_params) };

        if result.is_null() {
            return Err(InferenceError::GenerationFailed(
                "generate_image returned null".to_string(),
            ));
        }

        let mut images = Vec::new();
        let batch_count = params.batch_count.max(1) as usize;
        for i in 0..batch_count {
            let c_img = unsafe { &*result.add(i) };
            if c_img.data.is_null() {
                continue;
            }
            let len = (c_img.width * c_img.height * c_img.channel) as usize;
            let data = unsafe { std::slice::from_raw_parts(c_img.data, len) }.to_vec();
            match SdImage::from_raw(c_img.width, c_img.height, c_img.channel, data) {
                Ok(img) => images.push(img),
                Err(_) => continue,
            }
        }

        unsafe {
            free(result as *mut c_void);
        }

        if images.is_empty() {
            return Err(InferenceError::GenerationFailed(
                "No images generated".to_string(),
            ));
        }

        Ok(images)
    }

    fn generate_video(&self, params: VideoGenParams) -> InferenceResult<SdVideo> {
        let mut cache = self.contexts.lock().unwrap();
        let ctx = cache.get_or_create(&params.model_config, &self.base_config)?;

        let mut strings = CStringHolder::new();
        let mut c_params: CVidGenParams = unsafe { std::mem::zeroed() };
        unsafe { sd_vid_gen_params_init(&mut c_params) };

        c_params.prompt = strings.cstr(&params.prompt);
        c_params.negative_prompt = strings.cstr(&params.negative_prompt);
        c_params.clip_skip = params.clip_skip;
        c_params.width = params.width;
        c_params.height = params.height;
        c_params.strength = params.strength;
        c_params.seed = params.seed;
        c_params.video_frames = params.video_frames;
        c_params.vace_strength = params.vace_strength;
        c_params.moe_boundary = params.moe_boundary;

        c_params.sample_params = build_c_sample_params(&params.sample_params);

        c_params.init_image = params
            .init_image
            .as_ref()
            .map(|img| image_to_c(img))
            .unwrap_or_else(null_c_image);

        c_params.end_image = params
            .end_image
            .as_ref()
            .map(|img| image_to_c(img))
            .unwrap_or_else(null_c_image);

        let mut num_frames_out: c_int = 0;
        let mut frames_out: *mut CSdImage = ptr::null_mut();
        let mut audio_out: *mut CSdAudio = ptr::null_mut();
        let success = unsafe {
            generate_video(ctx, &c_params, &mut frames_out, &mut num_frames_out, &mut audio_out)
        };

        if !success || frames_out.is_null() || num_frames_out <= 0 {
            return Err(InferenceError::GenerationFailed(
                "generate_video returned null".to_string(),
            ));
        }

        let mut frames = Vec::new();
        for i in 0..num_frames_out as usize {
            let c_img = unsafe { &*frames_out.add(i) };
            if c_img.data.is_null() {
                tracing::warn!("generate_video: frame {} has null data", i);
                continue;
            }
            let len = (c_img.width * c_img.height * c_img.channel) as usize;
            let mut data = unsafe { std::slice::from_raw_parts(c_img.data, len) }.to_vec();

            // Debug: print first frame pixel stats
            if i == 0 {
                let pixels = (c_img.width * c_img.height) as usize;
                let ch = c_img.channel as usize;
                let mut mins = vec![255u8; ch];
                let mut maxs = vec![0u8; ch];
                let mut sums = vec![0u64; ch];
                for p in 0..pixels {
                    for c in 0..ch {
                        let v = data[p * ch + c];
                        mins[c] = mins[c].min(v);
                        maxs[c] = maxs[c].max(v);
                        sums[c] += v as u64;
                    }
                }
                tracing::info!(
                    "VAE frame0: {}x{} ch={} R=[{},{}] G=[{},{}] B=[{},{}] mean=[{:.1},{:.1},{:.1}] first10={:?}",
                    c_img.width, c_img.height, c_img.channel,
                    mins[0], maxs[0], mins[1], maxs[1], mins[2], maxs[2],
                    sums[0] as f64 / pixels as f64, sums[1] as f64 / pixels as f64, sums[2] as f64 / pixels as f64,
                    &data[..10.min(data.len())]
                );
            }

            // LTX-2.3 VAE outputs BGR channel order; swap to RGB for video encoding
            if c_img.channel == 3 {
                let pixels = (c_img.width * c_img.height) as usize;
                for i in 0..pixels {
                    let tmp = data[i * 3];
                    data[i * 3] = data[i * 3 + 2]; // B -> R
                    data[i * 3 + 2] = tmp;           // R -> B
                }
            }

            if let Ok(img) = SdImage::from_raw(c_img.width, c_img.height, c_img.channel, data) {
                frames.push(img);
            }
        }

        unsafe {
            free_sd_images(frames_out, num_frames_out);
            if !audio_out.is_null() {
                free_sd_audio(audio_out);
            }
        }

        Ok(SdVideo::new(frames, 16))
    }

    fn upscale(&self, image: SdImage, params: UpscaleParams) -> InferenceResult<SdImage> {
        let c_esrgan_path = CString::new(params.esrgan_path.as_str())
            .map_err(|e: NulError| InferenceError::InvalidParameter(e.to_string()))?;

        let upscaler_ctx = unsafe {
            new_upscaler_ctx(
                c_esrgan_path.as_ptr(),
                params.offload_to_cpu,
                params.direct,
                params.n_threads,
                params.tile_size,
                ptr::null(),  // backend
                ptr::null(),  // params_backend
            )
        };

        if upscaler_ctx.is_null() {
            return Err(InferenceError::ContextCreationFailed);
        }

        let c_input = image_to_c(&image);
        let c_result = unsafe { upscale(upscaler_ctx, c_input, params.upscale_factor) };

        unsafe {
            free_upscaler_ctx(upscaler_ctx);
        }

        if c_result.data.is_null() {
            return Err(InferenceError::GenerationFailed(
                "upscale returned null".to_string(),
            ));
        }

        let len = (c_result.width * c_result.height * c_result.channel) as usize;
        let data = unsafe { std::slice::from_raw_parts(c_result.data, len) }.to_vec();

        SdImage::from_raw(c_result.width, c_result.height, c_result.channel, data)
            .map_err(|e| InferenceError::ImageDecodeError(e.to_string()))
    }

    fn generate_3d_gaussian(&self, _params: Gaussian3DParams) -> InferenceResult<Gaussian3DOutput> {
        Err(InferenceError::BackendNotAvailable(
            "3D Gaussian generation is not supported in this version".to_string(),
        ))
    }
}

impl Drop for LocalBackend {
    fn drop(&mut self) {
        let mut cache = self.contexts.lock().unwrap();
        cache.clear();
    }
}

struct CStringHolder {
    strings: Vec<CString>,
}

impl CStringHolder {
    fn new() -> Self {
        Self { strings: Vec::new() }
    }

    fn cstr(&mut self, s: &str) -> *const c_char {
        let c = CString::new(s).unwrap_or_else(|_| CString::new("").unwrap());
        let ptr = c.as_ptr();
        self.strings.push(c);
        ptr
    }

    fn opt_cstr(&mut self, s: &Option<String>) -> *const c_char {
        match s {
            Some(s) if !s.is_empty() => self.cstr(s),
            _ => ptr::null(),
        }
    }
}

fn image_to_c(image: &SdImage) -> CSdImage {
    CSdImage {
        width: image.width,
        height: image.height,
        channel: image.channel,
        data: image.data.as_ptr() as *mut u8,
    }
}

fn null_c_image() -> CSdImage {
    CSdImage {
        width: 0,
        height: 0,
        channel: 0,
        data: ptr::null_mut(),
    }
}

fn build_c_sample_params(params: &SampleParams) -> CSampleParams {
    let mut c_params: CSampleParams = unsafe { std::mem::zeroed() };
    unsafe { sd_sample_params_init(&mut c_params) };

    c_params.guidance.txt_cfg = params.guidance.txt_cfg;
    if let Some(img_cfg) = params.guidance.img_cfg {
        c_params.guidance.img_cfg = img_cfg;
    }
    c_params.guidance.distilled_guidance = params.guidance.distilled_guidance;
    c_params.guidance.slg.layer_start = params.guidance.slg.layer_start;
    c_params.guidance.slg.layer_end = params.guidance.slg.layer_end;
    c_params.guidance.slg.scale = params.guidance.slg.scale;

    c_params.scheduler = match params.scheduler {
        Scheduler::Discrete => CScheduler::Discrete,
        Scheduler::Karras => CScheduler::Karras,
        Scheduler::Exponential => CScheduler::Exponential,
        Scheduler::Ays => CScheduler::Ays,
        Scheduler::Gits => CScheduler::Gits,
        Scheduler::SgmUniform => CScheduler::SgmUniform,
        Scheduler::Simple => CScheduler::Simple,
        Scheduler::Smoothstep => CScheduler::Smoothstep,
        Scheduler::KlOptimal => CScheduler::KlOptimal,
        Scheduler::Lcm => CScheduler::Lcm,
        Scheduler::BongTangent => CScheduler::BongTangent,
        Scheduler::Ltx2 => CScheduler::Ltx2,
    };
    c_params.sample_method = match params.sample_method {
        SampleMethod::Euler => CSampleMethod::Euler,
        SampleMethod::EulerA => CSampleMethod::EulerA,
        SampleMethod::Heun => CSampleMethod::Heun,
        SampleMethod::DPM2 => CSampleMethod::DPM2,
        SampleMethod::DPMPP2SA => CSampleMethod::DPMPP2SA,
        SampleMethod::DPMPP2M => CSampleMethod::DPMPP2M,
        SampleMethod::DPMPP2Mv2 => CSampleMethod::DPMPP2Mv2,
        SampleMethod::IPNDM => CSampleMethod::IPNDM,
        SampleMethod::IPNDMV => CSampleMethod::IPNDMV,
        SampleMethod::LCM => CSampleMethod::LCM,
        SampleMethod::DDIMTrailing => CSampleMethod::DDIMTrailing,
        SampleMethod::TCD => CSampleMethod::TCD,
        SampleMethod::ResMultistep => CSampleMethod::ResMultistep,
        SampleMethod::Res2S => CSampleMethod::Res2S,
        SampleMethod::ErSde => CSampleMethod::ErSde,
    };
    c_params.sample_steps = params.sample_steps;
    if let Some(eta) = params.eta {
        c_params.eta = eta;
    }
    c_params.shifted_timestep = params.shifted_timestep;
    if let Some(flow_shift) = params.flow_shift {
        c_params.flow_shift = flow_shift;
    }

    c_params
}

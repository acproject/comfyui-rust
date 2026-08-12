use crate::backend::InferenceBackend;
use crate::error::{InferenceError, InferenceResult};
use crate::image::{SdAudio, SdImage, SdVideo};
use crate::params::{
    H3Context, H3Mode, H3Params, ContextIrParams,
    ImageGenParams, UpscaleParams, VideoGenParams,
};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use serde::{Deserialize, Serialize};
use std::fs;
use std::sync::Arc;
use std::time::Duration;
use tempfile::TempDir;

/// Progress callback type: (step, total_steps, phase, message)
pub type FlashProgressCallback = Arc<dyn Fn(u32, u32, &str, Option<&str>) + Send + Sync>;

/// FlashAttn Bridge 配置
#[derive(Debug, Clone)]
pub struct FlashAttnConfig {
    /// Python Bridge 服务地址 (默认 http://127.0.0.1:8998)
    pub bridge_url: String,
    /// 请求超时时间（秒）
    pub timeout_sec: u64,
    /// 轮询间隔（毫秒）
    pub poll_interval_ms: u64,
    /// 自动启动 Python Bridge 服务（如果未运行）
    pub auto_start: bool,
    /// Python 解释器路径
    pub python_path: Option<String>,
    /// 模型根目录
    pub models_dir: Option<String>,
    /// GPU 设备 ID
    pub device_id: i32,
    /// 默认量化方式 (none, int8, int4, fp8)
    pub quantization: String,
}

impl Default for FlashAttnConfig {
    fn default() -> Self {
        Self {
            bridge_url: "http://127.0.0.1:8998".to_string(),
            timeout_sec: 900,  // 15 minutes for long video generation
            poll_interval_ms: 1000,
            auto_start: false,
            python_path: None,
            models_dir: None,
            device_id: 0,
            quantization: "int8".to_string(),
        }
    }
}

impl FlashAttnConfig {
    pub fn new(bridge_url: impl Into<String>) -> Self {
        Self {
            bridge_url: bridge_url.into(),
            ..Default::default()
        }
    }

    pub fn with_timeout(mut self, sec: u64) -> Self {
        self.timeout_sec = sec;
        self
    }

    pub fn with_models_dir(mut self, dir: impl Into<String>) -> Self {
        self.models_dir = Some(dir.into());
        self
    }

    pub fn with_quantization(mut self, quant: impl Into<String>) -> Self {
        self.quantization = quant.into();
        self
    }

    pub fn with_device(mut self, device_id: i32) -> Self {
        self.device_id = device_id;
        self
    }
}

// ========== Bridge Request/Response schemas ==========

#[derive(Debug, Deserialize)]
struct JobResponse {
    job_id: String,
    status: String,
}

#[derive(Debug, Deserialize)]
struct HealthResponse {
    status: String,
    model_loaded: bool,
    model_id: Option<String>,
    model_type: Option<String>,
    device: Option<String>,
    model_path: Option<String>,
    workflow: Option<String>,
}

#[derive(Debug, Serialize)]
struct LoadRequest {
    model_id: String,
    model_type: String,
    model_path: Option<String>,
    quantization: String,
    device_id: i32,
    dtype: String,
    gpu_memory_utilization: f32,
}

#[derive(Debug, Deserialize)]
struct LoadStatusResponse {
    job_id: String,
    status: String,
    progress: f64,
    message: Option<String>,
    error: Option<String>,
}

#[derive(Debug, Serialize)]
struct T2VARequest {
    prompt: String,
    negative_prompt: String,
    width: i32,
    height: i32,
    num_frames: i32,
    fps: i32,
    num_inference_steps: i32,
    guidance_scale: f64,
    seed: i64,
    audio_duration: Option<f64>,
    generate_sfx: bool,
    generate_bgm: bool,
    shift: Option<f64>,
}

#[derive(Debug, Serialize)]
struct I2VARequest {
    prompt: String,
    negative_prompt: String,
    ref_image_b64: String,
    width: i32,
    height: i32,
    num_frames: i32,
    fps: i32,
    num_inference_steps: i32,
    guidance_scale: f64,
    seed: i64,
    audio_duration: Option<f64>,
    generate_sfx: bool,
    generate_bgm: bool,
    shift: Option<f64>,
}

#[derive(Debug, Serialize)]
struct Ref2VARequest {
    prompt: String,
    negative_prompt: String,
    reference_images: Vec<String>,
    ref_video_b64: Option<String>,
    width: i32,
    height: i32,
    num_frames: i32,
    fps: i32,
    num_inference_steps: i32,
    guidance_scale: f64,
    seed: i64,
    audio_duration: Option<f64>,
    generate_sfx: bool,
    generate_bgm: bool,
    shift: Option<f64>,
}

#[derive(Debug, Deserialize)]
struct GenerationResultResponse {
    job_id: String,
    status: String,
    video_path: Option<String>,  // server-side path (not directly accessible)
    audio_path: Option<String>,
    duration_sec: Option<f64>,
    fps: Option<i32>,
    width: Option<i32>,
    height: Option<i32>,
    num_frames: Option<i32>,
    error: Option<String>,
    progress: Option<f64>,
    step: Option<u32>,
    total_steps: Option<u32>,
    phase: Option<String>,
    message: Option<String>,
}

#[derive(Debug, Serialize)]
struct ContextIrRequest {
    image_b64: Option<String>,
    video_frames_b64: Option<Vec<String>>,
    user_prompt: Option<String>,
    parse_sfx: bool,
    parse_bgm: bool,
}

#[derive(Debug, Deserialize)]
struct ContextIrResponse {
    subject: String,
    environment: String,
    style: String,
    camera_motion: String,
    sound_effects: Vec<String>,
    bgm: Option<String>,
    negative_prompt: Option<String>,
}

// ========== FlashAttnBackend ==========

pub struct FlashAttnBackend {
    config: FlashAttnConfig,
    client: reqwest::blocking::Client,
    model_loaded: std::sync::atomic::AtomicBool,
    progress_callback: Option<FlashProgressCallback>,
}

impl std::fmt::Debug for FlashAttnBackend {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FlashAttnBackend")
            .field("config", &self.config)
            .field("model_loaded", &self.model_loaded)
            .finish()
    }
}

impl FlashAttnBackend {
    pub fn new(config: FlashAttnConfig) -> Self {
        let client = reqwest::blocking::Client::builder()
            .timeout(Duration::from_secs(config.timeout_sec))
            .connect_timeout(Duration::from_secs(30))
            .build()
            .expect("Failed to create HTTP client");

        Self {
            config,
            client,
            model_loaded: std::sync::atomic::AtomicBool::new(false),
            progress_callback: None,
        }
    }

    pub fn with_progress_callback(mut self, cb: FlashProgressCallback) -> Self {
        self.progress_callback = Some(cb);
        self
    }

    fn url(&self, path: &str) -> String {
        format!("{}{}", self.config.bridge_url.trim_end_matches('/'), path)
    }

    /// 检查 Bridge 服务健康状态
    pub fn check_health(&self) -> InferenceResult<HealthResponse> {
        let resp = self.client
            .get(self.url("/health"))
            .send()
            .map_err(|e| InferenceError::ModelNotLoaded(format!("Bridge health check failed: {}", e)))?;

        if !resp.status().is_success() {
            return Err(InferenceError::ModelNotLoaded(
                format!("Bridge returned status: {}", resp.status())
            ));
        }

        let health: HealthResponse = resp.json()
            .map_err(|e| InferenceError::ModelNotLoaded(format!("Failed to parse health response: {}", e)))?;

        if health.model_loaded {
            self.model_loaded.store(true, std::sync::atomic::Ordering::SeqCst);
        }

        Ok(health)
    }

    /// 等待模型加载完成（轮询）
    fn wait_for_load(&self, job_id: &str) -> InferenceResult<()> {
        let timeout = Duration::from_secs(300); // 5 minutes for loading
        let start = std::time::Instant::now();
        let poll_interval = Duration::from_millis(500);

        loop {
            if start.elapsed() > timeout {
                return Err(InferenceError::ModelNotLoaded(
                    "Model loading timed out".to_string()
                ));
            }

            let resp = self.client
                .get(self.url(&format!("/load/status/{}", job_id)))
                .send()
                .map_err(|e| InferenceError::ModelNotLoaded(format!("Load status check failed: {}", e)))?;

            if resp.status().is_success() {
                let status: LoadStatusResponse = resp.json()
                    .map_err(|e| InferenceError::ModelNotLoaded(format!("Failed to parse load status: {}", e)))?;

                // Report loading progress
                if let Some(ref cb) = self.progress_callback {
                    let step = (status.progress * 100.0) as u32;
                    cb(step, 100, "loading", status.message.as_deref());
                }

                match status.status.as_str() {
                    "loaded" | "completed" => {
                        self.model_loaded.store(true, std::sync::atomic::Ordering::SeqCst);
                        return Ok(());
                    }
                    "failed" => {
                        return Err(InferenceError::ModelNotLoaded(
                            status.error.unwrap_or_else(|| "Unknown load error".to_string())
                        ));
                    }
                    _ => {
                        // pending or loading, continue polling
                    }
                }
            }

            std::thread::sleep(poll_interval);
        }
    }

    /// 加载模型到 Bridge（如果尚未加载）
    pub fn ensure_model_loaded(&self) -> InferenceResult<()> {
        if self.model_loaded.load(std::sync::atomic::Ordering::SeqCst) {
            return Ok(());
        }

        // 先检查健康状态
        if let Ok(health) = self.check_health() {
            if health.model_loaded {
                self.model_loaded.store(true, std::sync::atomic::Ordering::SeqCst);
                return Ok(());
            }
        }

        // 发起加载请求
        let model_path = self.config.models_dir.as_ref()
            .map(|d| format!("{}/HunyuanVideoAudio", d));

        let req = LoadRequest {
            model_id: "MiniMax-H3".to_string(),
            model_type: "t2va".to_string(),
            model_path,
            quantization: self.config.quantization.clone(),
            device_id: self.config.device_id,
            dtype: "bf16".to_string(),
            gpu_memory_utilization: 0.85,
        };

        let resp = self.client
            .post(self.url("/load"))
            .json(&req)
            .send()
            .map_err(|e| InferenceError::ModelNotLoaded(format!("Failed to send load request: {}", e)))?;

        if !resp.status().is_success() {
            let err_text = resp.text().unwrap_or_default();
            return Err(InferenceError::ModelNotLoaded(
                format!("Failed to start model loading: {}", err_text)
            ));
        }

        let job_resp: JobResponse = resp.json()
            .map_err(|e| InferenceError::ModelNotLoaded(format!("Failed to parse load response: {}", e)))?;

        // 等待加载完成
        self.wait_for_load(&job_resp.job_id)?;

        Ok(())
    }

    fn encode_image_to_b64(&self, img: &SdImage) -> InferenceResult<String> {
        let png_bytes = img.to_png_bytes()
            .map_err(|e| InferenceError::InvalidParameter(format!("Failed to encode image: {}", e)))?;
        Ok(BASE64.encode(&png_bytes))
    }

    /// 轮询生成结果直到完成
    fn poll_generation_result(&self, job_id: &str) -> InferenceResult<GenerationResultResponse> {
        let timeout = Duration::from_secs(self.config.timeout_sec);
        let start = std::time::Instant::now();
        let poll_interval = Duration::from_millis(self.config.poll_interval_ms);
        let mut last_reported_step: u32 = 0;

        loop {
            if start.elapsed() > timeout {
                return Err(InferenceError::GenerationFailed(
                    "Generation timed out".to_string()
                ));
            }

            let resp = self.client
                .get(self.url(&format!("/generate/result/{}", job_id)))
                .send()
                .map_err(|e| InferenceError::GenerationFailed(format!("Result poll failed: {}", e)))?;

            if !resp.status().is_success() {
                let err_text = resp.text().unwrap_or_default();
                return Err(InferenceError::GenerationFailed(
                    format!("Generation status check failed: {}", err_text)
                ));
            }

            let result: GenerationResultResponse = resp.json()
                .map_err(|e| InferenceError::GenerationFailed(format!("Failed to parse result: {}", e)))?;

            // Report progress if callback is set and step changed
            if let Some(ref cb) = self.progress_callback {
                let step = result.step.unwrap_or(0);
                let total = result.total_steps.unwrap_or(0);
                if step != last_reported_step || result.status == "completed" {
                    let phase = result.phase.as_deref().unwrap_or("generating");
                    let msg = result.message.as_deref();
                    cb(step, total, phase, msg);
                    last_reported_step = step;
                }
            }

            match result.status.as_str() {
                "completed" => return Ok(result),
                "failed" => {
                    return Err(InferenceError::GenerationFailed(
                        result.error.unwrap_or_else(|| "Unknown generation error".to_string())
                    ));
                }
                _ => {
                    // pending or processing, continue
                }
            }

            std::thread::sleep(poll_interval);
        }
    }

    /// 下载媒体文件（视频或音频）到临时目录
    fn download_media(&self, job_id: &str, media_type: &str, tmp_dir: &TempDir) -> InferenceResult<Option<std::path::PathBuf>> {
        let url = self.url(&format!("/media/{}/{}", media_type, job_id));
        let resp = self.client
            .get(&url)
            .send()
            .map_err(|e| InferenceError::GenerationFailed(format!("Failed to download {}: {}", media_type, e)))?;

        if !resp.status().is_success() {
            if resp.status() == reqwest::StatusCode::NOT_FOUND {
                return Ok(None);
            }
            return Err(InferenceError::GenerationFailed(
                format!("Failed to download {}: status {}", media_type, resp.status())
            ));
        }

        let bytes = resp.bytes()
            .map_err(|e| InferenceError::GenerationFailed(format!("Failed to read {} bytes: {}", media_type, e)))?;

        let ext = if media_type == "video" { "mp4" } else { "wav" };
        let file_path = tmp_dir.path().join(format!("{}.{}", media_type, ext));
        fs::write(&file_path, &bytes)
            .map_err(|e| InferenceError::GenerationFailed(format!("Failed to save {}: {}", media_type, e)))?;

        Ok(Some(file_path))
    }

    /// 提交 T2VA 任务并等待结果，返回 SdVideo
    fn do_t2va(&self, params: &H3Params) -> InferenceResult<SdVideo> {
        self.ensure_model_loaded()?;

        let req = T2VARequest {
            prompt: params.prompt.clone(),
            negative_prompt: params.negative_prompt.clone(),
            width: params.width,
            height: params.height,
            num_frames: params.num_frames,
            fps: params.fps,
            num_inference_steps: params.num_inference_steps,
            guidance_scale: params.guidance_scale,
            seed: params.seed,
            audio_duration: params.audio_duration,
            generate_sfx: params.generate_sfx,
            generate_bgm: params.generate_bgm,
            shift: params.shift,
        };

        let resp = self.client
            .post(self.url("/generate/t2va"))
            .json(&req)
            .send()
            .map_err(|e| InferenceError::GenerationFailed(format!("T2VA request failed: {}", e)))?;

        if !resp.status().is_success() {
            let err_text = resp.text().unwrap_or_default();
            return Err(InferenceError::GenerationFailed(
                format!("T2VA generation failed to start: {}", err_text)
            ));
        }

        let job_resp: JobResponse = resp.json()
            .map_err(|e| InferenceError::GenerationFailed(format!("Failed to parse T2VA response: {}", e)))?;

        let result = self.poll_generation_result(&job_resp.job_id)?;
        let fps = result.fps.unwrap_or(params.fps);

        // Download and decode
        self.download_and_decode(&job_resp.job_id, fps)
    }

    /// 提交 I2VA 任务并等待结果，返回 SdVideo
    fn do_i2va(&self, params: &H3Params) -> InferenceResult<SdVideo> {
        self.ensure_model_loaded()?;

        let ref_image = params.reference_images.first()
            .ok_or_else(|| InferenceError::InvalidParameter("I2VA requires a reference image".to_string()))?;
        let ref_b64 = self.encode_image_to_b64(ref_image)?;

        let req = I2VARequest {
            prompt: params.prompt.clone(),
            negative_prompt: params.negative_prompt.clone(),
            ref_image_b64: ref_b64,
            width: params.width,
            height: params.height,
            num_frames: params.num_frames,
            fps: params.fps,
            num_inference_steps: params.num_inference_steps,
            guidance_scale: params.guidance_scale,
            seed: params.seed,
            audio_duration: params.audio_duration,
            generate_sfx: params.generate_sfx,
            generate_bgm: params.generate_bgm,
            shift: params.shift,
        };

        let resp = self.client
            .post(self.url("/generate/i2va"))
            .json(&req)
            .send()
            .map_err(|e| InferenceError::GenerationFailed(format!("I2VA request failed: {}", e)))?;

        if !resp.status().is_success() {
            let err_text = resp.text().unwrap_or_default();
            return Err(InferenceError::GenerationFailed(
                format!("I2VA generation failed to start: {}", err_text)
            ));
        }

        let job_resp: JobResponse = resp.json()
            .map_err(|e| InferenceError::GenerationFailed(format!("Failed to parse I2VA response: {}", e)))?;

        let result = self.poll_generation_result(&job_resp.job_id)?;
        let fps = result.fps.unwrap_or(params.fps);

        self.download_and_decode(&job_resp.job_id, fps)
    }

    /// 提交 Ref2VA 任务并等待结果，返回 SdVideo
    fn do_ref2va(&self, params: &H3Params) -> InferenceResult<SdVideo> {
        self.ensure_model_loaded()?;

        let ref_images_b64: InferenceResult<Vec<String>> = params.reference_images.iter()
            .map(|img| self.encode_image_to_b64(img))
            .collect();
        let mut ref_images_b64 = ref_images_b64?;

        let ref_video_b64 = params.reference_video.as_ref().and_then(|v| {
            v.frames.first().and_then(|f| self.encode_image_to_b64(f).ok())
        });

        if let Some(ref video) = params.reference_video {
            for frame in video.frames.iter().take(2) {
                if let Ok(b64) = self.encode_image_to_b64(frame) {
                    ref_images_b64.push(b64);
                }
            }
        }

        let req = Ref2VARequest {
            prompt: params.prompt.clone(),
            negative_prompt: params.negative_prompt.clone(),
            reference_images: ref_images_b64,
            ref_video_b64,
            width: params.width,
            height: params.height,
            num_frames: params.num_frames,
            fps: params.fps,
            num_inference_steps: params.num_inference_steps,
            guidance_scale: params.guidance_scale,
            seed: params.seed,
            audio_duration: params.audio_duration,
            generate_sfx: params.generate_sfx,
            generate_bgm: params.generate_bgm,
            shift: params.shift,
        };

        let resp = self.client
            .post(self.url("/generate/ref2va"))
            .json(&req)
            .send()
            .map_err(|e| InferenceError::GenerationFailed(format!("Ref2VA request failed: {}", e)))?;

        if !resp.status().is_success() {
            let err_text = resp.text().unwrap_or_default();
            return Err(InferenceError::GenerationFailed(
                format!("Ref2VA generation failed to start: {}", err_text)
            ));
        }

        let job_resp: JobResponse = resp.json()
            .map_err(|e| InferenceError::GenerationFailed(format!("Failed to parse Ref2VA response: {}", e)))?;

        let result = self.poll_generation_result(&job_resp.job_id)?;
        let fps = result.fps.unwrap_or(params.fps);

        self.download_and_decode(&job_resp.job_id, fps)
    }

    /// Context-IR 调用
    fn do_context_ir(&self, params: &ContextIrParams) -> InferenceResult<ContextIrResponse> {
        self.ensure_model_loaded()?;

        let image_b64 = if let Some(ref img) = params.image {
            Some(self.encode_image_to_b64(img)?)
        } else {
            None
        };

        let video_frames_b64 = if let Some(ref video) = params.video {
            let frames: InferenceResult<Vec<String>> = video.frames.iter()
                .take(4)
                .map(|f| self.encode_image_to_b64(f))
                .collect();
            Some(frames?)
        } else {
            None
        };

        let req = ContextIrRequest {
            image_b64,
            video_frames_b64,
            user_prompt: params.user_prompt.clone(),
            parse_sfx: params.parse_sfx,
            parse_bgm: params.parse_bgm,
        };

        let resp = self.client
            .post(self.url("/context-ir"))
            .json(&req)
            .send()
            .map_err(|e| InferenceError::GenerationFailed(format!("Context-IR request failed: {}", e)))?;

        if !resp.status().is_success() {
            let err_text = resp.text().unwrap_or_default();
            return Err(InferenceError::GenerationFailed(
                format!("Context-IR failed: {}", err_text)
            ));
        }

        resp.json::<ContextIrResponse>()
            .map_err(|e| InferenceError::GenerationFailed(format!("Failed to parse Context-IR response: {}", e)))
    }

    /// 下载媒体文件并解码为 SdVideo
    fn download_and_decode(&self, job_id: &str, fps: i32) -> InferenceResult<SdVideo> {
        let tmp_dir = TempDir::new()
            .map_err(|e| InferenceError::GenerationFailed(format!("Failed to create temp dir: {}", e)))?;

        let video_path = self.download_media(job_id, "video", &tmp_dir)?;
        let audio_path = self.download_media(job_id, "audio", &tmp_dir)?;

        // 解码视频帧
        let frames = if let Some(ref vpath) = video_path {
            if vpath.exists() {
                SdVideo::decode_with_ffmpeg(vpath, fps)
                    .map(|v| v.frames)
                    .unwrap_or_default()
            } else {
                Vec::new()
            }
        } else {
            Vec::new()
        };

        // 解码音频
        let audio = if let Some(ref apath) = audio_path {
            if apath.exists() {
                let wav_bytes = fs::read(apath)
                    .map_err(|e| InferenceError::GenerationFailed(format!("Failed to read audio file: {}", e)))?;
                SdAudio::from_wav_bytes(&wav_bytes).ok()
            } else {
                None
            }
        } else {
            None
        };

        Ok(SdVideo::new(frames, fps, audio))
    }
}

impl InferenceBackend for FlashAttnBackend {
    fn supports_image_generation(&self) -> bool {
        false
    }

    fn supports_video_generation(&self) -> bool {
        false
    }

    fn supports_audio_video_generation(&self) -> bool {
        true
    }

    fn supports_context_ir(&self) -> bool {
        true
    }

    fn generate_image(&self, _params: ImageGenParams) -> InferenceResult<Vec<SdImage>> {
        Err(InferenceError::BackendNotAvailable(
            "FlashAttnBackend does not support image generation directly. Use local/remote backend.".to_string()
        ))
    }

    fn generate_video(&self, _params: VideoGenParams) -> InferenceResult<SdVideo> {
        Err(InferenceError::BackendNotAvailable(
            "FlashAttnBackend does not support plain video generation. Use generate_av for H3 models.".to_string()
        ))
    }

    fn upscale(&self, _image: SdImage, _params: UpscaleParams) -> InferenceResult<SdImage> {
        Err(InferenceError::BackendNotAvailable("FlashAttnBackend does not support upscaling".to_string()))
    }

    fn generate_av(&self, params: H3Params) -> InferenceResult<SdVideo> {
        match params.mode {
            H3Mode::T2VA | H3Mode::SFX | H3Mode::Audio => self.do_t2va(&params),
            H3Mode::I2VA => self.do_i2va(&params),
            H3Mode::Ref2VA => self.do_ref2va(&params),
            H3Mode::MR2VA => {
                if !params.reference_images.is_empty() {
                    self.do_i2va(&params)
                } else if params.reference_video.is_some() {
                    self.do_ref2va(&params)
                } else {
                    self.do_t2va(&params)
                }
            }
        }
    }

    fn context_ir(&self, params: ContextIrParams) -> InferenceResult<H3Context> {
        let resp = self.do_context_ir(&params)?;

        Ok(H3Context {
            subject: resp.subject,
            environment: resp.environment,
            style: resp.style,
            camera_motion: resp.camera_motion,
            sound_effects: resp.sound_effects,
            bgm: resp.bgm,
            negative_prompt: resp.negative_prompt,
        })
    }
}

impl Drop for FlashAttnBackend {
    fn drop(&mut self) {
        // Best-effort unload
        let _ = self.client.post(self.url("/unload")).send();
    }
}

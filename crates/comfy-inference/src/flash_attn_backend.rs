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
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};
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
    /// Python 解释器路径（None = 自动检测venv）
    pub python_path: Option<String>,
    /// 项目根目录（None = 自动检测）
    pub project_root: Option<String>,
    /// Bridge 日志文件路径（None = 自动生成到/tmp）
    pub bridge_log_file: Option<String>,
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
            auto_start: true,   // 默认自动启动
            python_path: None,
            project_root: None,
            bridge_log_file: None,
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

    pub fn with_auto_start(mut self, auto: bool) -> Self {
        self.auto_start = auto;
        self
    }

    pub fn with_python_path(mut self, path: impl Into<String>) -> Self {
        self.python_path = Some(path.into());
        self
    }

    pub fn with_project_root(mut self, root: impl Into<String>) -> Self {
        self.project_root = Some(root.into());
        self
    }

    /// 从 bridge_url 解析端口号
    fn bridge_port(&self) -> u16 {
        // http://127.0.0.1:8998 -> 8998
        self.bridge_url
            .rsplit(':')
            .next()
            .and_then(|p| p.parse().ok())
            .unwrap_or(8998)
    }
}

// ========== Bridge Request/Response schemas ==========

#[derive(Debug, Deserialize)]
pub struct JobResponse {
    pub job_id: String,
    pub status: String,
}

#[derive(Debug, Deserialize)]
pub struct HealthResponse {
    pub status: String,
    pub model_loaded: bool,
    pub model_id: Option<String>,
    pub model_type: Option<String>,
    pub device: Option<String>,
    pub model_path: Option<String>,
    pub workflow: Option<String>,
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
    /// 我们自己启动的Bridge子进程PID（用于崩溃检测）
    bridge_pid: Mutex<Option<u32>>,
    /// 防止并发auto-start的互斥锁
    start_lock: Mutex<()>,
}

impl std::fmt::Debug for FlashAttnBackend {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FlashAttnBackend")
            .field("config", &self.config)
            .field("model_loaded", &self.model_loaded)
            .field("bridge_pid", &self.bridge_pid.lock().map(|p| *p).unwrap_or(None))
            .finish()
    }
}

impl FlashAttnBackend {
    pub fn new(config: FlashAttnConfig) -> Self {
        let client = reqwest::blocking::Client::builder()
            .timeout(Duration::from_secs(config.timeout_sec))
            .connect_timeout(Duration::from_secs(10))
            .build()
            .expect("Failed to create HTTP client");

        Self {
            config,
            client,
            model_loaded: std::sync::atomic::AtomicBool::new(false),
            progress_callback: None,
            bridge_pid: Mutex::new(None),
            start_lock: Mutex::new(()),
        }
    }

    pub fn with_progress_callback(mut self, cb: FlashProgressCallback) -> Self {
        self.progress_callback = Some(cb);
        self
    }

    /// Explicitly shut down the Bridge process (if we started it).
    /// Call this before application exit for clean shutdown.
    pub fn shutdown(&self) {
        // Best-effort unload
        let unload_client = reqwest::blocking::Client::builder()
            .timeout(Duration::from_secs(5))
            .connect_timeout(Duration::from_secs(2))
            .build();
        if let Ok(client) = unload_client {
            let _ = client.post(self.url("/unload")).send();
        }
        // Kill our child process
        self.kill_bridge_child();
    }

    fn url(&self, path: &str) -> String {
        format!("{}{}", self.config.bridge_url.trim_end_matches('/'), path)
    }

    // ========== Auto-Start Bridge Process Management ==========

    /// 自动检测项目根目录
    fn detect_project_root() -> Option<PathBuf> {
        // 1. 检查环境变量
        if let Ok(root) = std::env::var("FLASH_ATTN_PROJECT_ROOT") {
            let p = PathBuf::from(root);
            if p.join("flash_attn_bridge").is_dir() {
                return Some(p);
            }
        }
        if let Ok(root) = std::env::var("PROJECT_ROOT") {
            let p = PathBuf::from(root);
            if p.join("flash_attn_bridge").is_dir() {
                return Some(p);
            }
        }

        // 2. 检查当前工作目录
        if let Ok(cwd) = std::env::current_dir() {
            if cwd.join("flash_attn_bridge").is_dir() {
                return Some(cwd);
            }
            // 检查 cwd/.. 
            if let Some(parent) = cwd.parent() {
                if parent.join("flash_attn_bridge").is_dir() {
                    return Some(parent.to_path_buf());
                }
            }
        }

        // 3. 检查可执行文件所在目录向上查找
        if let Ok(exe) = std::env::current_exe() {
            let mut dir = exe.parent().map(|p| p.to_path_buf());
            for _ in 0..5 {
                if let Some(d) = &dir {
                    if d.join("flash_attn_bridge").is_dir() {
                        return Some(d.clone());
                    }
                    dir = d.parent().map(|p| p.to_path_buf());
                } else {
                    break;
                }
            }
        }

        // 4. 检查常见路径
        let common_paths = [
            "/home/acproject/workspace/python_projects/flash_attn_v100",
        ];
        for p in &common_paths {
            let pb = PathBuf::from(p);
            if pb.join("flash_attn_bridge").is_dir() {
                return Some(pb);
            }
        }

        None
    }

    /// 查找 Python 解释器路径
    fn find_python(&self, project_root: &Path) -> PathBuf {
        // 1. 配置中指定的路径
        if let Some(ref py) = self.config.python_path {
            return PathBuf::from(py);
        }

        // 2. 项目 venv
        let venv_py = project_root.join("venv-cu128").join("bin").join("python");
        if venv_py.exists() {
            return venv_py;
        }

        // 3. venv 通用名
        for venv_name in &["venv", ".venv", "env", ".env"] {
            let py = project_root.join(venv_name).join("bin").join("python");
            if py.exists() {
                return py;
            }
        }

        // 4. 系统 python3
        PathBuf::from("python3")
    }

    /// 终止已存在的 Bridge 子进程
    fn kill_bridge_child(&self) {
        let mut guard = self.bridge_pid.lock().unwrap();
        if let Some(pid) = *guard {
            if Path::new(&format!("/proc/{}", pid)).exists() {
                let _ = Command::new("kill").arg("-TERM").arg(pid.to_string()).status();
                std::thread::sleep(Duration::from_secs(2));
                if Path::new(&format!("/proc/{}", pid)).exists() {
                    let _ = Command::new("kill").arg("-KILL").arg(pid.to_string()).status();
                    std::thread::sleep(Duration::from_millis(500));
                }
            }
            // Clean up PID file
            let _ = Self::detect_project_root().map(|root| {
                let _ = fs::remove_file(root.join("flash_attn_bridge").join(".bridge.pid"));
            });
            *guard = None;
        }
    }

    /// 检查端口是否已被占用（可能是外部启动的Bridge）
    fn is_port_open(&self) -> bool {
        use std::net::TcpStream;
        let port = self.config.bridge_port();
        // 从URL解析host
        let host = self.config.bridge_url
            .strip_prefix("http://")
            .or_else(|| self.config.bridge_url.strip_prefix("https://"))
            .unwrap_or("127.0.0.1")
            .split(':')
            .next()
            .unwrap_or("127.0.0.1");
        TcpStream::connect_timeout(
            &format!("{}:{}", host, port).parse().unwrap_or_else(|_| "127.0.0.1:8998".parse().unwrap()),
            Duration::from_millis(500),
        ).is_ok()
    }

    /// 启动 Bridge 子进程
    fn spawn_bridge_process(&self) -> InferenceResult<()> {
        let project_root = self.config.project_root
            .as_ref()
            .map(PathBuf::from)
            .or_else(Self::detect_project_root)
            .ok_or_else(|| InferenceError::ModelNotLoaded(
                "Cannot detect project root for auto-start. \
                 Please set FLASH_ATTN_PROJECT_ROOT env var or use with_project_root().".to_string()
            ))?;

        let python = self.find_python(&project_root);
        let port = self.config.bridge_port();

        // 创建日志文件
        let log_path = if let Some(ref lf) = self.config.bridge_log_file {
            PathBuf::from(lf)
        } else {
            PathBuf::from(format!("/tmp/flash_attn_bridge_{}.log", std::process::id()))
        };

        let log_file = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log_path)
            .map_err(|e| InferenceError::ModelNotLoaded(
                format!("Failed to open log file {}: {}", log_path.display(), e)
            ))?;

        let log_file_err = log_file.try_clone()
            .map_err(|e| InferenceError::ModelNotLoaded(
                format!("Failed to clone log file handle: {}", e)
            ))?;

        // 设置环境变量
        let mut envs = vec![
            ("FA_BRIDGE_HOST".to_string(), "127.0.0.1".to_string()),
            ("FA_BRIDGE_PORT".to_string(), port.to_string()),
            ("FA_QUANTIZATION".to_string(), self.config.quantization.clone()),
            ("FA_TRANSFORMER_DEVICES".to_string(), format!("cuda:{}", self.config.device_id)),
            ("FA_VAE_DEVICE".to_string(), format!("cuda:{}", self.config.device_id)),
            ("FA_VAE_FP16".to_string(), "1".to_string()),
            ("FA_USE_CACHE".to_string(), "1".to_string()),
            ("FA_AUTO_LOAD".to_string(), "0".to_string()),
            ("OMP_NUM_THREADS".to_string(), "4".to_string()),
            ("MKL_NUM_THREADS".to_string(), "4".to_string()),
            ("PYTORCH_CUDA_ALLOC_CONF".to_string(), "expandable_segments:True".to_string()),
            ("TOKENIZERS_PARALLELISM".to_string(), "false".to_string()),
        ];

        // 设置 CUDA_VISIBLE_DEVICES
        envs.push(("CUDA_VISIBLE_DEVICES".to_string(), self.config.device_id.to_string()));

        // 继承 PATH 和 LD_LIBRARY_PATH
        for key in &["PATH", "LD_LIBRARY_PATH", "HOME", "USER", "LANG", "LC_ALL"] {
            if let Ok(val) = std::env::var(key) {
                envs.push((key.to_string(), val));
            }
        }

        // 如果指定了模型目录
        if let Some(ref models_dir) = self.config.models_dir {
            let model_path = format!("{}/HunyuanVideoAudio", models_dir);
            if Path::new(&model_path).exists() {
                envs.push(("FA_MODEL_PATH".to_string(), model_path));
            }
        }

        // 先杀掉可能残留的子进程（由本实例启动的）
        self.kill_bridge_child();

        // 如果PID文件存在且对应进程还在运行，先杀掉
        let pid_file = project_root.join("flash_attn_bridge").join(".bridge.pid");
        if pid_file.exists() {
            if let Ok(pid_str) = fs::read_to_string(&pid_file) {
                if let Ok(old_pid) = pid_str.trim().parse::<u32>() {
                    if Path::new(&format!("/proc/{}", old_pid)).exists() {
                        tracing::warn!("Killing stale Bridge process (PID {})", old_pid);
                        let _ = Command::new("kill").arg("-TERM").arg(old_pid.to_string()).status();
                        std::thread::sleep(Duration::from_secs(2));
                        if Path::new(&format!("/proc/{}", old_pid)).exists() {
                            let _ = Command::new("kill").arg("-KILL").arg(old_pid.to_string()).status();
                            std::thread::sleep(Duration::from_millis(500));
                        }
                    }
                }
            }
            let _ = fs::remove_file(&pid_file);
        }

        // 启动进程
        let mut cmd = Command::new(&python);
        cmd.current_dir(&project_root)
            .arg("-m")
            .arg("flash_attn_bridge.server")
            .arg("--host")
            .arg("127.0.0.1")
            .arg("--port")
            .arg(port.to_string())
            .arg("--quantization")
            .arg(&self.config.quantization)
            .arg("--transformer-devices")
            .arg(format!("cuda:{}", self.config.device_id))
            .arg("--vae-device")
            .arg(format!("cuda:{}", self.config.device_id))
            .stdin(Stdio::null())
            .stdout(Stdio::from(log_file))
            .stderr(Stdio::from(log_file_err));

        for (k, v) in &envs {
            cmd.env(k, v);
        }

        // 添加PYTHONPATH: 确保project_root在路径中
        let existing_pythonpath = std::env::var("PYTHONPATH").unwrap_or_default();
        let new_pythonpath = if existing_pythonpath.is_empty() {
            project_root.to_string_lossy().to_string()
        } else {
            format!("{}:{}", project_root.display(), existing_pythonpath)
        };
        cmd.env("PYTHONPATH", &new_pythonpath);

        tracing::info!(
            "Starting FlashAttn Bridge: {} -m flash_attn_bridge.server --port {} (log: {})",
            python.display(), port, log_path.display()
        );

        let mut child = cmd.spawn()
            .map_err(|e| InferenceError::ModelNotLoaded(
                format!("Failed to spawn Bridge process ({}): {}. \
                         Make sure venv-cu128 exists and flash_attn_bridge module is installed.",
                        python.display(), e)
            ))?;

        let pid = child.id();
        *self.bridge_pid.lock().unwrap() = Some(pid);

        // 写入PID文件（与start.sh兼容）
        let _ = fs::write(&pid_file, pid.to_string());

        // Spawn a background thread to wait for the child and reap it when it exits
        // This prevents zombie processes while allowing the Bridge to persist
        let reaper_pid_file = pid_file.clone();
        std::thread::spawn(move || {
            let _ = child.wait();
            // Process exited - clean up PID file
            let _ = fs::remove_file(&reaper_pid_file);
            tracing::warn!("FlashAttn Bridge process (PID {}) exited", pid);
        });

        tracing::info!("FlashAttn Bridge started with PID {}", pid);

        // 等待服务就绪
        let startup_timeout = Duration::from_secs(30);
        let start = std::time::Instant::now();
        let poll_interval = Duration::from_millis(500);

        // 创建一个短超时的client用于健康检查
        let health_client = reqwest::blocking::Client::builder()
            .timeout(Duration::from_secs(3))
            .connect_timeout(Duration::from_secs(2))
            .build()
            .unwrap();

        loop {
            if start.elapsed() > startup_timeout {
                // 检查子进程是否还在
                let still_alive = Path::new(&format!("/proc/{}", pid)).exists();
                if !still_alive {
                    *self.bridge_pid.lock().unwrap() = None;
                    let _ = fs::remove_file(&pid_file);
                    return Err(InferenceError::ModelNotLoaded(
                        format!("Bridge process exited during startup. Check log: {}", log_path.display())
                    ));
                }
                return Err(InferenceError::ModelNotLoaded(
                    format!("Bridge did not become ready within {}s. Check log: {}",
                            startup_timeout.as_secs(), log_path.display())
                ));
            }

            // 检查子进程是否崩溃
            if !Path::new(&format!("/proc/{}", pid)).exists() {
                *self.bridge_pid.lock().unwrap() = None;
                let _ = fs::remove_file(&pid_file);
                return Err(InferenceError::ModelNotLoaded(
                    format!("Bridge process exited during startup. Check log: {}", log_path.display())
                ));
            }

            // 尝试健康检查
            match health_client.get(self.url("/health")).send() {
                Ok(resp) if resp.status().is_success() => {
                    tracing::info!("FlashAttn Bridge is ready (took {:.1}s)", start.elapsed().as_secs_f64());
                    // 重置model_loaded标志（新进程需要重新加载模型）
                    self.model_loaded.store(false, std::sync::atomic::Ordering::SeqCst);
                    return Ok(());
                }
                _ => {}
            }

            std::thread::sleep(poll_interval);
        }
    }

    /// 确保Bridge服务正在运行 —— 如果未运行且auto_start=true则自动启动
    fn ensure_bridge_running(&self) -> InferenceResult<()> {
        // 快速检查：端口是否开放？
        if self.is_port_open() {
            // 端口开放了，做一次健康检查确认
            let health_client = reqwest::blocking::Client::builder()
                .timeout(Duration::from_secs(5))
                .connect_timeout(Duration::from_secs(3))
                .build()
                .unwrap();
            if health_client.get(self.url("/health")).send()
                .and_then(|r| Ok(r.status().is_success())).unwrap_or(false)
            {
                return Ok(());
            }
        }

        // Bridge未运行
        if !self.config.auto_start {
            return Err(InferenceError::ModelNotLoaded(format!(
                "FlashAttn Bridge is not running at {} and auto_start is disabled. \
                 Please start it manually: {}/flash_attn_bridge/start.sh --daemon",
                self.config.bridge_url,
                Self::detect_project_root().map(|p| p.display().to_string()).unwrap_or_else(|| "<project_root>".to_string())
            )));
        }

        // 获取启动锁（防止多个线程同时启动）
        let _lock = self.start_lock.lock().unwrap();

        // 双重检查：获得锁后再检查一次，可能其他线程已经启动了
        if self.is_port_open() {
            let health_client = reqwest::blocking::Client::builder()
                .timeout(Duration::from_secs(5))
                .connect_timeout(Duration::from_secs(3))
                .build()
                .unwrap();
            if health_client.get(self.url("/health")).send()
                .and_then(|r| Ok(r.status().is_success())).unwrap_or(false)
            {
                return Ok(());
            }
        }

        // 启动Bridge
        self.spawn_bridge_process()
    }

    /// 检查子进程是否还活着，如果死了清理状态
    fn check_child_alive(&self) -> bool {
        let mut guard = self.bridge_pid.lock().unwrap();
        if let Some(pid) = *guard {
            if Path::new(&format!("/proc/{}", pid)).exists() {
                true
            } else {
                tracing::warn!("FlashAttn Bridge process (PID {}) exited unexpectedly", pid);
                *guard = None;
                self.model_loaded.store(false, std::sync::atomic::Ordering::SeqCst);
                // Clean up PID file
                let _ = Self::detect_project_root().map(|root| {
                    let _ = fs::remove_file(root.join("flash_attn_bridge").join(".bridge.pid"));
                });
                false
            }
        } else {
            false
        }
    }

    /// 带自动重启的HTTP请求包装
    fn with_auto_restart<F, T>(&self, f: F) -> InferenceResult<T>
    where
        F: Fn() -> InferenceResult<T>,
    {
        // 第一次尝试
        match f() {
            Ok(v) => Ok(v),
            Err(e) => {
                let err_msg = format!("{}", e);
                let is_connection_err = err_msg.contains("connection")
                    || err_msg.contains("Connect")
                    || err_msg.contains("refused")
                    || err_msg.contains("reset")
                    || err_msg.contains("timeout")
                    || err_msg.contains("tcp");

                // 如果是连接错误且我们有auto_start，尝试重启Bridge并重试一次
                if is_connection_err && self.config.auto_start {
                    // 检查子进程是否死了
                    self.check_child_alive();

                    tracing::warn!("Bridge request failed ({}), attempting restart...", err_msg);
                    self.model_loaded.store(false, std::sync::atomic::Ordering::SeqCst);

                    // 重启
                    if let Err(restart_err) = self.ensure_bridge_running() {
                        tracing::error!("Bridge restart failed: {}", restart_err);
                        return Err(e);
                    }

                    // 重试一次
                    f().map_err(|retry_e| {
                        InferenceError::GenerationFailed(format!(
                            "Request failed after bridge restart. Original error: {}. Retry error: {}",
                            e, retry_e
                        ))
                    })
                } else {
                    Err(e)
                }
            }
        }
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
        // 确保Bridge服务正在运行
        self.ensure_bridge_running()?;

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

        let resp = self.with_auto_restart(|| {
            self.client
                .post(self.url("/load"))
                .json(&req)
                .send()
                .map_err(|e| InferenceError::ModelNotLoaded(format!("Failed to send load request: {}", e)))
        })?;

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

        let resp = self.with_auto_restart(|| {
            self.client
                .post(self.url("/generate/t2va"))
                .json(&req)
                .send()
                .map_err(|e| InferenceError::GenerationFailed(format!("T2VA request failed: {}", e)))
        })?;

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

        let resp = self.with_auto_restart(|| {
            self.client
                .post(self.url("/generate/i2va"))
                .json(&req)
                .send()
                .map_err(|e| InferenceError::GenerationFailed(format!("I2VA request failed: {}", e)))
        })?;

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

        let resp = self.with_auto_restart(|| {
            self.client
                .post(self.url("/generate/ref2va"))
                .json(&req)
                .send()
                .map_err(|e| InferenceError::GenerationFailed(format!("Ref2VA request failed: {}", e)))
        })?;

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

    /// 提交 SFX (音效生成) 任务 - 只生成音效，默认短时长
    fn do_sfx(&self, params: &H3Params) -> InferenceResult<SdVideo> {
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
            audio_duration: params.audio_duration.or(Some(5.0)),
            generate_sfx: true,
            generate_bgm: false,
            shift: params.shift,
        };

        let resp = self.with_auto_restart(|| {
            self.client
                .post(self.url("/generate/sfx"))
                .json(&req)
                .send()
                .map_err(|e| InferenceError::GenerationFailed(format!("SFX request failed: {}", e)))
        })?;

        if !resp.status().is_success() {
            let err_text = resp.text().unwrap_or_default();
            return Err(InferenceError::GenerationFailed(
                format!("SFX generation failed to start: {}", err_text)
            ));
        }

        let job_resp: JobResponse = resp.json()
            .map_err(|e| InferenceError::GenerationFailed(format!("Failed to parse SFX response: {}", e)))?;

        let result = self.poll_generation_result(&job_resp.job_id)?;
        let fps = result.fps.unwrap_or(params.fps);

        self.download_and_decode(&job_resp.job_id, fps)
    }

    /// 提交 BGM/Audio (背景音乐生成) 任务 - 只生成背景音乐
    fn do_audio(&self, params: &H3Params) -> InferenceResult<SdVideo> {
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
            audio_duration: params.audio_duration.or(Some(10.0)),
            generate_sfx: false,
            generate_bgm: true,
            shift: params.shift,
        };

        let resp = self.with_auto_restart(|| {
            self.client
                .post(self.url("/generate/audio"))
                .json(&req)
                .send()
                .map_err(|e| InferenceError::GenerationFailed(format!("Audio/BGM request failed: {}", e)))
        })?;

        if !resp.status().is_success() {
            let err_text = resp.text().unwrap_or_default();
            return Err(InferenceError::GenerationFailed(
                format!("Audio/BGM generation failed to start: {}", err_text)
            ));
        }

        let job_resp: JobResponse = resp.json()
            .map_err(|e| InferenceError::GenerationFailed(format!("Failed to parse Audio response: {}", e)))?;

        let result = self.poll_generation_result(&job_resp.job_id)?;
        let fps = result.fps.unwrap_or(params.fps);

        self.download_and_decode(&job_resp.job_id, fps)
    }

    /// 提交 MR2VA (多参考视频生成) 任务
    fn do_mr2va(&self, params: &H3Params) -> InferenceResult<SdVideo> {
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

        let resp = self.with_auto_restart(|| {
            self.client
                .post(self.url("/generate/mr2va"))
                .json(&req)
                .send()
                .map_err(|e| InferenceError::GenerationFailed(format!("MR2VA request failed: {}", e)))
        })?;

        if !resp.status().is_success() {
            let err_text = resp.text().unwrap_or_default();
            return Err(InferenceError::GenerationFailed(
                format!("MR2VA generation failed to start: {}", err_text)
            ));
        }

        let job_resp: JobResponse = resp.json()
            .map_err(|e| InferenceError::GenerationFailed(format!("Failed to parse MR2VA response: {}", e)))?;

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

        let resp = self.with_auto_restart(|| {
            self.client
                .post(self.url("/context-ir"))
                .json(&req)
                .send()
                .map_err(|e| InferenceError::GenerationFailed(format!("Context-IR request failed: {}", e)))
        })?;

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
            H3Mode::T2VA => self.do_t2va(&params),
            H3Mode::SFX => self.do_sfx(&params),
            H3Mode::Audio => self.do_audio(&params),
            H3Mode::I2VA => self.do_i2va(&params),
            H3Mode::Ref2VA => self.do_ref2va(&params),
            H3Mode::MR2VA => self.do_mr2va(&params),
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
        // Best-effort unload model (free GPU memory) using a short timeout
        let unload_client = reqwest::blocking::Client::builder()
            .timeout(Duration::from_secs(5))
            .connect_timeout(Duration::from_secs(2))
            .build();
        if let Ok(client) = unload_client {
            let _ = client.post(self.url("/unload")).send();
        }
        // Note: We do NOT kill the Bridge process on Drop.
        // The Bridge persists across node calls for reuse.
        // It will be reaped by the background wait thread when it eventually exits.
        // Use stop.sh or `kill <pid>` to stop it manually.
    }
}

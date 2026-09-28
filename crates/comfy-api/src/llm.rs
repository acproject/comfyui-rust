use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmConfig {
    #[serde(default = "default_mode")]
    pub mode: String,
    #[serde(default = "default_cli_path")]
    pub cli_path: String,
    #[serde(default = "default_extra_args")]
    pub extra_args: String,
    #[serde(default = "default_api_url")]
    pub api_url: String,
    #[serde(default)]
    pub api_key: Option<String>,
    #[serde(default = "default_model")]
    pub model: String,
    #[serde(default = "default_max_tokens")]
    pub max_tokens: u32,
    #[serde(default = "default_temperature")]
    pub temperature: f64,
    #[serde(default = "default_top_p")]
    pub top_p: f64,
    #[serde(default = "default_system_prompt")]
    pub system_prompt: String,
    /// In "local" mode, retry with the Python HF fallback when llama-cli
    /// cannot load the model (HF format dirs, missing gguf support, etc.).
    #[serde(default = "default_python_fallback")]
    pub python_fallback: bool,
    /// Python interpreter for the LLM fallback (None = auto-detect).
    #[serde(default)]
    pub python_path: Option<String>,
    /// Directory containing comfy_fallback/llm_generate.py.
    #[serde(default)]
    pub python_script_dir: Option<String>,
    #[serde(default = "default_python_dtype")]
    pub python_dtype: String,
}

impl Default for LlmConfig {
    fn default() -> Self {
        Self {
            mode: default_mode(),
            cli_path: default_cli_path(),
            extra_args: default_extra_args(),
            api_url: default_api_url(),
            api_key: None,
            model: default_model(),
            max_tokens: default_max_tokens(),
            temperature: default_temperature(),
            top_p: default_top_p(),
            system_prompt: default_system_prompt(),
            python_fallback: default_python_fallback(),
            python_path: None,
            python_script_dir: None,
            python_dtype: default_python_dtype(),
        }
    }
}

impl LlmConfig {
    pub fn from_env() -> Self {
        let mut config = Self::default();

        if let Ok(val) = std::env::var("COMFY_LLM_MODE") {
            config.mode = val;
        }
        if let Ok(val) = std::env::var("COMFY_LLM_CLI_PATH") {
            config.cli_path = val;
        }
        if let Ok(val) = std::env::var("COMFY_LLM_EXTRA_ARGS") {
            config.extra_args = val;
        }
        if let Ok(val) = std::env::var("COMFY_LLM_API_URL") {
            config.api_url = val;
        }
        if let Ok(val) = std::env::var("COMFY_LLM_API_KEY") {
            config.api_key = Some(val);
        }
        if let Ok(val) = std::env::var("COMFY_LLM_MODEL") {
            config.model = val;
        }
        if let Ok(val) = std::env::var("COMFY_LLM_MAX_TOKENS") {
            if let Ok(n) = val.parse() {
                config.max_tokens = n;
            }
        }
        if let Ok(val) = std::env::var("COMFY_LLM_TEMPERATURE") {
            if let Ok(t) = val.parse() {
                config.temperature = t;
            }
        }
        if let Ok(val) = std::env::var("COMFY_LLM_TOP_P") {
            if let Ok(p) = val.parse() {
                config.top_p = p;
            }
        }
        if let Ok(val) = std::env::var("COMFY_LLM_PYTHON_FALLBACK") {
            config.python_fallback = val == "1" || val.eq_ignore_ascii_case("true");
        }
        if let Ok(val) = std::env::var("COMFY_LLM_PYTHON_PATH") {
            config.python_path = Some(val);
        }
        if let Ok(val) = std::env::var("COMFY_LLM_PYTHON_SCRIPT_DIR") {
            config.python_script_dir = Some(val);
        }

        config
    }

    pub fn to_executor_config(&self) -> serde_json::Value {
        serde_json::json!({
            "mode": self.mode,
            "cli_path": self.cli_path,
            "extra_args": self.extra_args,
            "api_url": self.api_url,
            "api_key": self.api_key,
            "model": self.model,
            "max_tokens": self.max_tokens,
            "temperature": self.temperature,
            "top_p": self.top_p,
            "system_prompt": self.system_prompt,
            "python_fallback": self.python_fallback,
            "python_path": self.python_path,
            "python_script_dir": self.python_script_dir,
            "python_dtype": self.python_dtype,
        })
    }

    /// Fill in machine-local paths that were left empty or point at
    /// non-existent files, using the workspace layout as the source of truth.
    pub fn resolve_local_paths(&mut self, workspace_root: Option<&std::path::Path>) {
        if !std::path::Path::new(&self.cli_path).exists() {
            if let Some(root) = workspace_root {
                let llama_cli = root
                    .join("cpp/llama.cpp/build/bin/llama-cli");
                if llama_cli.exists() {
                    self.cli_path = llama_cli.to_string_lossy().to_string();
                }
            }
        }
        if self.python_script_dir.as_deref().map(|p| p.is_empty()).unwrap_or(true) {
            if let Some(root) = workspace_root {
                let dir = root.join("py/flash_attn_v100/comfy_fallback");
                if dir.is_dir() {
                    self.python_script_dir = Some(dir.to_string_lossy().to_string());
                }
            }
        }
    }
}

fn default_mode() -> String {
    "local".to_string()
}

fn default_cli_path() -> String {
    "/home/acproject/workspace/rust_projects/comfyui-rust/cpp/llama.cpp/build/bin/llama-cli".to_string()
}

fn default_extra_args() -> String {
    "".to_string()
}

fn default_api_url() -> String {
    "http://127.0.0.1:8080".to_string()
}

fn default_model() -> String {
    "default".to_string()
}

fn default_max_tokens() -> u32 {
    512
}

fn default_temperature() -> f64 {
    0.7
}

fn default_top_p() -> f64 {
    0.9
}

fn default_system_prompt() -> String {
    "".to_string()
}

fn default_python_fallback() -> bool {
    true
}

fn default_python_dtype() -> String {
    "auto".to_string()
}

pub struct LlmService {
    config: Arc<RwLock<LlmConfig>>,
}

impl LlmService {
    pub fn new(config: LlmConfig) -> Self {
        Self {
            config: Arc::new(RwLock::new(config)),
        }
    }

    pub async fn get_config(&self) -> LlmConfig {
        self.config.read().await.clone()
    }

    pub async fn set_config(&self, new_config: LlmConfig) {
        *self.config.write().await = new_config;
    }

    pub fn config_arc(&self) -> Arc<RwLock<LlmConfig>> {
        self.config.clone()
    }
}

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
        })
    }
}

fn default_mode() -> String {
    "local".to_string()
}

fn default_cli_path() -> String {
    "/home/acproject/workspace/rust_projects/comfyui-rust/cpp/llama.cpp-qwen3-omni/build/bin/llama-cli".to_string()
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

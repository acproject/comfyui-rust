use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComfyConfig {
    pub server: ServerConfig,
    pub models: ModelPathsConfig,
    pub inference: InferenceConfig,
    pub output: OutputConfig,
    #[serde(default)]
    pub extra: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    #[serde(default = "default_host")]
    pub host: String,
    #[serde(default = "default_port")]
    pub port: u16,
    #[serde(default)]
    pub cors_origins: Vec<String>,
    #[serde(default)]
    pub static_dir: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelPathsConfig {
    #[serde(default = "default_models_dir")]
    pub base_dir: String,
    #[serde(default = "default_checkpoints_dir")]
    pub checkpoints: String,
    #[serde(default = "default_clip_dir")]
    pub clip: String,
    #[serde(default = "default_vae_dir")]
    pub vae: String,
    #[serde(default = "default_lora_dir")]
    pub lora: String,
    #[serde(default = "default_controlnet_dir")]
    pub controlnet: String,
    #[serde(default = "default_upscale_dir")]
    pub upscale: String,
    #[serde(default = "default_embeddings_dir")]
    pub embeddings: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InferenceConfig {
    #[serde(default = "default_backend")]
    pub backend: String,
    #[serde(default = "default_n_threads")]
    pub n_threads: u32,
    #[serde(default)]
    pub vae_decode_only: bool,
    #[serde(default = "default_true")]
    pub free_params_immediately: bool,
    #[serde(default = "default_true")]
    pub enable_mmap: bool,
    #[serde(default)]
    pub flash_attn: bool,
    #[serde(default)]
    pub offload_params_to_cpu: bool,
    #[serde(default)]
    pub remote_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutputConfig {
    #[serde(default = "default_output_dir")]
    pub dir: String,
    #[serde(default = "default_true")]
    pub save_metadata: bool,
    #[serde(default)]
    pub format: String,
}

impl Default for ComfyConfig {
    fn default() -> Self {
        Self {
            server: ServerConfig::default(),
            models: ModelPathsConfig::default(),
            inference: InferenceConfig::default(),
            output: OutputConfig::default(),
            extra: HashMap::new(),
        }
    }
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            host: default_host(),
            port: default_port(),
            cors_origins: Vec::new(),
            static_dir: None,
        }
    }
}

impl Default for ModelPathsConfig {
    fn default() -> Self {
        Self {
            base_dir: default_models_dir(),
            checkpoints: default_checkpoints_dir(),
            clip: default_clip_dir(),
            vae: default_vae_dir(),
            lora: default_lora_dir(),
            controlnet: default_controlnet_dir(),
            upscale: default_upscale_dir(),
            embeddings: default_embeddings_dir(),
        }
    }
}

impl Default for InferenceConfig {
    fn default() -> Self {
        Self {
            backend: default_backend(),
            n_threads: default_n_threads(),
            vae_decode_only: false,
            free_params_immediately: true,
            enable_mmap: true,
            flash_attn: false,
            offload_params_to_cpu: false,
            remote_url: None,
        }
    }
}

impl Default for OutputConfig {
    fn default() -> Self {
        Self {
            dir: default_output_dir(),
            save_metadata: true,
            format: "png".to_string(),
        }
    }
}

fn default_host() -> String {
    "127.0.0.1".to_string()
}

fn default_port() -> u16 {
    8188
}

fn default_models_dir() -> String {
    "models".to_string()
}

fn default_checkpoints_dir() -> String {
    "checkpoints".to_string()
}

fn default_clip_dir() -> String {
    "clip".to_string()
}

fn default_vae_dir() -> String {
    "vae".to_string()
}

fn default_lora_dir() -> String {
    "loras".to_string()
}

fn default_controlnet_dir() -> String {
    "controlnet".to_string()
}

fn default_upscale_dir() -> String {
    "upscale_models".to_string()
}

fn default_embeddings_dir() -> String {
    "embeddings".to_string()
}

fn default_backend() -> String {
    "local".to_string()
}

fn default_n_threads() -> u32 {
    std::thread::available_parallelism()
        .map(|n| n.get() as u32)
        .unwrap_or(4)
}

fn default_output_dir() -> String {
    "output".to_string()
}

fn default_true() -> bool {
    true
}

impl ComfyConfig {
    pub fn load(path: &Path) -> Result<Self, ConfigError> {
        if !path.exists() {
            return Ok(Self::default());
        }

        let content = std::fs::read_to_string(path).map_err(ConfigError::IoError)?;
        let config: Self =
            serde_json::from_str(&content).map_err(ConfigError::ParseError)?;

        Ok(config)
    }

    pub fn save(&self, path: &Path) -> Result<(), ConfigError> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(ConfigError::IoError)?;
        }

        let content =
            serde_json::to_string_pretty(self).map_err(ConfigError::SerializeError)?;

        std::fs::write(path, content).map_err(ConfigError::IoError)?;

        Ok(())
    }

    pub fn from_env() -> Self {
        let mut config = Self::default();

        if let Ok(host) = std::env::var("COMFY_HOST") {
            config.server.host = host;
        }
        if let Ok(port) = std::env::var("COMFY_PORT") {
            if let Ok(p) = port.parse() {
                config.server.port = p;
            }
        }
        if let Ok(dir) = std::env::var("COMFY_OUTPUT_DIR") {
            config.output.dir = dir;
        }
        if let Ok(dir) = std::env::var("COMFY_MODELS_DIR") {
            config.models.base_dir = dir;
        }
        if let Ok(backend) = std::env::var("COMFY_BACKEND") {
            config.inference.backend = backend;
        }
        if let Ok(url) = std::env::var("COMFY_REMOTE_URL") {
            config.inference.remote_url = Some(url);
        }
        if let Ok(n) = std::env::var("COMFY_THREADS") {
            if let Ok(t) = n.parse() {
                config.inference.n_threads = t;
            }
        }

        config
    }

    pub fn resolve_model_path(&self, model_type: &str, filename: &str) -> PathBuf {
        let sub_dir = match model_type {
            "checkpoints" => &self.models.checkpoints,
            "clip" | "text_encoders" => &self.models.clip,
            "vae" => &self.models.vae,
            "loras" => &self.models.lora,
            "controlnet" => &self.models.controlnet,
            "upscale_models" => &self.models.upscale,
            "embeddings" => &self.models.embeddings,
            _ => "misc",
        };

        PathBuf::from(&self.models.base_dir).join(sub_dir).join(filename)
    }

    pub fn address(&self) -> String {
        format!("{}:{}", self.server.host, self.server.port)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("IO error: {0}")]
    IoError(std::io::Error),
    #[error("Parse error: {0}")]
    ParseError(serde_json::Error),
    #[error("Serialize error: {0}")]
    SerializeError(serde_json::Error),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = ComfyConfig::default();
        assert_eq!(config.server.host, "127.0.0.1");
        assert_eq!(config.server.port, 8188);
        assert_eq!(config.inference.backend, "local");
    }

    #[test]
    fn test_config_roundtrip() {
        let config = ComfyConfig::default();
        let dir = std::env::temp_dir().join("comfy_test_config");
        let path = dir.join("config.json");

        config.save(&path).unwrap();
        let loaded = ComfyConfig::load(&path).unwrap();

        assert_eq!(loaded.server.host, config.server.host);
        assert_eq!(loaded.server.port, config.server.port);

        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn test_resolve_model_path() {
        let config = ComfyConfig::default();
        let path = config.resolve_model_path("checkpoints", "model.safetensors");
        assert!(path.to_string_lossy().contains("checkpoints"));
        assert!(path.to_string_lossy().contains("model.safetensors"));
    }
}

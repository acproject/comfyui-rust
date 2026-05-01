use crate::agent::{AgentConfig, AgentService};
use crate::config::ComfyConfig;
use crate::images::ImageStore;
use crate::queue::PromptQueue;
use crate::ws::WsBroadcaster;
use comfy_executor::{Executor, NodeRegistry};
use comfy_inference::NullBackend;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;

pub struct AppState {
    pub executor: Arc<Executor>,
    pub queue: Arc<PromptQueue>,
    pub broadcaster: WsBroadcaster,
    pub registry: Arc<Mutex<NodeRegistry>>,
    pub images: Arc<ImageStore>,
    pub input_dir: PathBuf,
    pub custom_nodes_dir: PathBuf,
    pub models_dir: PathBuf,
    pub config: Arc<std::sync::RwLock<ComfyConfig>>,
    pub config_path: Arc<PathBuf>,
    pub agent: Arc<AgentService>,
}

fn create_agent_service() -> Arc<AgentService> {
    let config = AgentConfig::from_env();
    Arc::new(AgentService::new(config))
}

impl AppState {
    pub fn new(registry: NodeRegistry) -> Self {
        let output_dir = std::env::var("COMFY_OUTPUT_DIR")
            .unwrap_or_else(|_| "output".to_string());
        Self::with_output_dir(registry, output_dir)
    }

    pub fn with_output_dir(registry: NodeRegistry, output_dir: String) -> Self {
        Self::build(registry, Some(output_dir), None)
    }

    pub fn with_config(registry: NodeRegistry, config: ComfyConfig, config_path: PathBuf) -> Self {
        let output_dir = config.output.dir.clone();
        Self::build(registry, Some(output_dir), Some((config, config_path)))
    }

    pub fn with_executor(registry: NodeRegistry, executor: Executor) -> Self {
        let input_dir = PathBuf::from(std::env::var("COMFY_INPUT_DIR")
            .unwrap_or_else(|_| "input".to_string()));
        std::fs::create_dir_all(&input_dir).ok();

        let custom_nodes_dir = PathBuf::from(std::env::var("COMFY_CUSTOM_NODES_DIR")
            .unwrap_or_else(|_| "custom_nodes".to_string()));
        std::fs::create_dir_all(&custom_nodes_dir).ok();

        let config = ComfyConfig::from_env();
        let models_dir = PathBuf::from(&config.models.base_dir);
        std::fs::create_dir_all(&models_dir).ok();

        let config_path = Self::default_config_path();

        let registry = Arc::new(Mutex::new(registry));
        let queue = Arc::new(PromptQueue::new());
        let broadcaster = WsBroadcaster::new();
        let images = Arc::new(ImageStore::new("output"));

        Self {
            executor: Arc::new(executor),
            queue,
            broadcaster,
            registry,
            images,
            input_dir,
            custom_nodes_dir,
            models_dir,
            config: Arc::new(std::sync::RwLock::new(config)),
            config_path: Arc::new(config_path),
            agent: create_agent_service(),
        }
    }

    pub fn with_inference_backend(
        registry: NodeRegistry,
        backend: Arc<dyn comfy_inference::InferenceBackend>,
    ) -> Self {
        let input_dir = PathBuf::from(std::env::var("COMFY_INPUT_DIR")
            .unwrap_or_else(|_| "input".to_string()));
        std::fs::create_dir_all(&input_dir).ok();

        let custom_nodes_dir = PathBuf::from(std::env::var("COMFY_CUSTOM_NODES_DIR")
            .unwrap_or_else(|_| "custom_nodes".to_string()));
        std::fs::create_dir_all(&custom_nodes_dir).ok();

        let config = ComfyConfig::from_env();
        let models_dir = PathBuf::from(&config.models.base_dir);
        std::fs::create_dir_all(&models_dir).ok();

        let output_dir = std::env::var("COMFY_OUTPUT_DIR")
            .unwrap_or_else(|_| "output".to_string());
        let executor = Arc::new(Executor::new(registry.clone(), backend));
        let registry = Arc::new(Mutex::new(registry));
        let queue = Arc::new(PromptQueue::new());
        let broadcaster = WsBroadcaster::new();
        let images = Arc::new(ImageStore::new(output_dir));

        let config_path = Self::default_config_path();

        Self {
            executor,
            queue,
            broadcaster,
            registry,
            images,
            input_dir,
            custom_nodes_dir,
            models_dir,
            config: Arc::new(std::sync::RwLock::new(config)),
            config_path: Arc::new(config_path),
            agent: create_agent_service(),
        }
    }

    fn build(
        registry: NodeRegistry,
        output_dir: Option<String>,
        config_with_path: Option<(ComfyConfig, PathBuf)>,
    ) -> Self {
        let (config, config_path) = config_with_path.unwrap_or_else(|| {
            let c = ComfyConfig::from_env();
            let p = Self::default_config_path();
            (c, p)
        });

        let output_dir = output_dir.unwrap_or_else(|| config.output.dir.clone());

        let input_dir = PathBuf::from(std::env::var("COMFY_INPUT_DIR")
            .unwrap_or_else(|_| "input".to_string()));
        std::fs::create_dir_all(&input_dir).ok();

        let custom_nodes_dir = PathBuf::from(std::env::var("COMFY_CUSTOM_NODES_DIR")
            .unwrap_or_else(|_| "custom_nodes".to_string()));
        std::fs::create_dir_all(&custom_nodes_dir).ok();

        let models_dir = PathBuf::from(&config.models.base_dir);
        std::fs::create_dir_all(&models_dir).ok();

        let executor = Arc::new(Executor::new(registry.clone(), Arc::new(NullBackend)));
        let registry = Arc::new(Mutex::new(registry));
        let queue = Arc::new(PromptQueue::new());
        let broadcaster = WsBroadcaster::new();
        let images = Arc::new(ImageStore::new(output_dir));

        Self {
            executor,
            queue,
            broadcaster,
            registry,
            images,
            input_dir,
            custom_nodes_dir,
            models_dir,
            config: Arc::new(std::sync::RwLock::new(config)),
            config_path: Arc::new(config_path),
            agent: create_agent_service(),
        }
    }

    pub fn default_config_path() -> PathBuf {
        let config_dir = std::env::var("COMFY_CONFIG_DIR")
            .unwrap_or_else(|_| "config".to_string());
        PathBuf::from(config_dir).join("config.json")
    }
}

impl Clone for AppState {
    fn clone(&self) -> Self {
        Self {
            executor: self.executor.clone(),
            queue: self.queue.clone(),
            broadcaster: self.broadcaster.clone(),
            registry: self.registry.clone(),
            images: self.images.clone(),
            input_dir: self.input_dir.clone(),
            custom_nodes_dir: self.custom_nodes_dir.clone(),
            models_dir: self.models_dir.clone(),
            config: self.config.clone(),
            config_path: self.config_path.clone(),
            agent: self.agent.clone(),
        }
    }
}

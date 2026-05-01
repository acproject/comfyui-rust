use crate::agent::{AgentConfig, AgentService};
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
        let input_dir = PathBuf::from(std::env::var("COMFY_INPUT_DIR")
            .unwrap_or_else(|_| "input".to_string()));
        std::fs::create_dir_all(&input_dir).ok();

        let custom_nodes_dir = PathBuf::from(std::env::var("COMFY_CUSTOM_NODES_DIR")
            .unwrap_or_else(|_| "custom_nodes".to_string()));
        std::fs::create_dir_all(&custom_nodes_dir).ok();

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
            agent: create_agent_service(),
        }
    }

    pub fn with_executor(registry: NodeRegistry, executor: Executor) -> Self {
        let input_dir = PathBuf::from(std::env::var("COMFY_INPUT_DIR")
            .unwrap_or_else(|_| "input".to_string()));
        std::fs::create_dir_all(&input_dir).ok();

        let custom_nodes_dir = PathBuf::from(std::env::var("COMFY_CUSTOM_NODES_DIR")
            .unwrap_or_else(|_| "custom_nodes".to_string()));
        std::fs::create_dir_all(&custom_nodes_dir).ok();

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

        let output_dir = std::env::var("COMFY_OUTPUT_DIR")
            .unwrap_or_else(|_| "output".to_string());
        let executor = Arc::new(Executor::new(registry.clone(), backend));
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
            agent: create_agent_service(),
        }
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
            agent: self.agent.clone(),
        }
    }
}

use crate::agent::{AgentConfig, AgentService};
use crate::config::ComfyConfig;
use crate::database::{Database, SharedDatabase};
use crate::download_tracker::{DownloadTracker, SharedDownloadTracker};
use crate::images::ImageStore;
use crate::queue::PromptQueue;
use crate::ws::WsBroadcaster;
use comfy_executor::{Executor, NodeRegistry};
use comfy_inference::NullBackend;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;

#[cfg(feature = "local-ffi")]
use comfy_inference::{LocalBackend, ContextConfig};
use comfy_inference::{CliBackend, CliBackendConfig};

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
    pub download_tracker: SharedDownloadTracker,
    pub db: SharedDatabase,
}

fn create_download_tracker() -> SharedDownloadTracker {
    Arc::new(Mutex::new(DownloadTracker::new()))
}

fn create_database(config_dir: &str) -> SharedDatabase {
    let db_path = std::path::Path::new(config_dir).join("comfyui.db");
    match Database::open(&db_path) {
        Ok(db) => Arc::new(db),
        Err(e) => {
            tracing::error!("Failed to open database at {}: {}, using in-memory", db_path.display(), e);
            Arc::new(Database::open_in_memory().expect("Failed to create in-memory database"))
        }
    }
}

fn load_agent_config_from_db(db: &Database) -> AgentConfig {
    match db.get::<AgentConfig>("agent_config") {
        Ok(Some(config)) => {
            tracing::info!("Loaded agent config from database");
            config
        }
        Ok(None) => AgentConfig::from_env(),
        Err(e) => {
            tracing::warn!("Failed to load agent config from database: {}, using defaults", e);
            AgentConfig::from_env()
        }
    }
}

fn config_dir_from_path(config_path: &PathBuf) -> String {
    config_path.parent()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|| "config".to_string())
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
        let config_dir = config_dir_from_path(&config_path);
        let db = create_database(&config_dir);
        let agent_config = load_agent_config_from_db(&db);

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
            agent: Arc::new(AgentService::new(agent_config)),
            download_tracker: create_download_tracker(),
            db,
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
        let config_dir = config_dir_from_path(&config_path);
        let db = create_database(&config_dir);
        let agent_config = load_agent_config_from_db(&db);

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
            agent: Arc::new(AgentService::new(agent_config)),
            download_tracker: create_download_tracker(),
            db,
        }
    }

    #[cfg(feature = "local-ffi")]
    pub fn with_local_backend(registry: NodeRegistry, config: ComfyConfig, config_path: PathBuf) -> Self {
        let models_dir = PathBuf::from(&config.models.base_dir);
        std::fs::create_dir_all(&models_dir).ok();

        let ctx_config = ContextConfig {
            n_threads: config.inference.n_threads as i32,
            vae_decode_only: config.inference.vae_decode_only,
            free_params_immediately: config.inference.free_params_immediately,
            enable_mmap: config.inference.enable_mmap,
            flash_attn: config.inference.flash_attn,
            offload_params_to_cpu: config.inference.offload_params_to_cpu,
            ..ContextConfig::default()
        };

        let backend = match LocalBackend::new(ctx_config) {
            Ok(b) => {
                tracing::info!("LocalBackend created successfully");
                Arc::new(b) as Arc<dyn comfy_inference::InferenceBackend>
            }
            Err(e) => {
                tracing::error!("Failed to create LocalBackend: {}, falling back to NullBackend", e);
                Arc::new(NullBackend) as Arc<dyn comfy_inference::InferenceBackend>
            }
        };

        let output_dir = config.output.dir.clone();
        let input_dir = PathBuf::from(std::env::var("COMFY_INPUT_DIR")
            .unwrap_or_else(|_| "input".to_string()));
        std::fs::create_dir_all(&input_dir).ok();

        let custom_nodes_dir = PathBuf::from(std::env::var("COMFY_CUSTOM_NODES_DIR")
            .unwrap_or_else(|_| "custom_nodes".to_string()));
        std::fs::create_dir_all(&custom_nodes_dir).ok();

        let executor = Arc::new(Executor::new(registry.clone(), backend));
        let registry = Arc::new(Mutex::new(registry));
        let queue = Arc::new(PromptQueue::new());
        let broadcaster = WsBroadcaster::new();
        let images = Arc::new(ImageStore::new(output_dir));

        let config_dir = config_dir_from_path(&config_path);
        let db = create_database(&config_dir);
        let agent_config = load_agent_config_from_db(&db);

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
            agent: Arc::new(AgentService::new(agent_config)),
            download_tracker: create_download_tracker(),
            db,
        }
    }

    pub fn with_cli_backend(registry: NodeRegistry, config: ComfyConfig, config_path: PathBuf) -> Self {
        let models_dir = PathBuf::from(&config.models.base_dir);
        std::fs::create_dir_all(&models_dir).ok();

        let sd_cli_path = config.inference.sd_cli_path.clone()
            .or_else(|| std::env::var("SD_CLI_PATH").ok())
            .or_else(|| {
                let manifest_dir = config_path.parent()?;
                let workspace_root = manifest_dir.parent()?;
                let possible_dirs = [
                    "cpp/stable-diffusion-cpp",
                    "cpp/stable-diffusion.cpp",
                ];
                for dir in &possible_dirs {
                    let cli_path = workspace_root.join(dir).join("build/bin/sd-cli");
                    if cli_path.exists() {
                        tracing::info!("Auto-detected sd-cli at {}", cli_path.display());
                        return Some(cli_path.to_string_lossy().to_string());
                    }
                }
                None
            })
            .unwrap_or_else(|| "sd-cli".to_string());

        let cli_config = CliBackendConfig::new(&sd_cli_path)
            .with_threads(config.inference.n_threads as i32)
            .with_flash_attn(config.inference.flash_attn)
            .with_offload_to_cpu(config.inference.offload_params_to_cpu)
            .with_verbose(true)
            .with_output_dir(&config.output.dir);

        let backend = match CliBackend::new(cli_config) {
            Ok(b) => {
                tracing::info!("CliBackend created with sd-cli at '{}'", sd_cli_path);
                Arc::new(b) as Arc<dyn comfy_inference::InferenceBackend>
            }
            Err(e) => {
                tracing::error!("Failed to create CliBackend: {}, falling back to NullBackend", e);
                Arc::new(NullBackend) as Arc<dyn comfy_inference::InferenceBackend>
            }
        };

        let output_dir = config.output.dir.clone();
        let input_dir = PathBuf::from(std::env::var("COMFY_INPUT_DIR")
            .unwrap_or_else(|_| "input".to_string()));
        std::fs::create_dir_all(&input_dir).ok();

        let custom_nodes_dir = PathBuf::from(std::env::var("COMFY_CUSTOM_NODES_DIR")
            .unwrap_or_else(|_| "custom_nodes".to_string()));
        std::fs::create_dir_all(&custom_nodes_dir).ok();

        let executor = Arc::new(Executor::new(registry.clone(), backend));
        let registry = Arc::new(Mutex::new(registry));
        let queue = Arc::new(PromptQueue::new());
        let broadcaster = WsBroadcaster::new();
        let images = Arc::new(ImageStore::new(output_dir));

        let config_dir = config_dir_from_path(&config_path);
        let db = create_database(&config_dir);
        let agent_config = load_agent_config_from_db(&db);

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
            agent: Arc::new(AgentService::new(agent_config)),
            download_tracker: create_download_tracker(),
            db,
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

        let config_dir = config_dir_from_path(&config_path);
        let db = create_database(&config_dir);
        let agent_config = load_agent_config_from_db(&db);

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
            agent: Arc::new(AgentService::new(agent_config)),
            download_tracker: create_download_tracker(),
            db,
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
            download_tracker: self.download_tracker.clone(),
            db: self.db.clone(),
        }
    }
}

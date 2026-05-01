use comfy_api::{AppState, ComfyConfig, ComfyServer};
use comfy_executor::builtin_nodes;
use comfy_executor::NodeRegistry;
use std::net::SocketAddr;
use std::path::Path;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let config_dir = std::env::var("COMFY_CONFIG_DIR")
        .unwrap_or_else(|_| "config".to_string());
    let config_path = Path::new(&config_dir).join("config.json");

    std::fs::create_dir_all(&config_dir).ok();

    let config = ComfyConfig::load(&config_path)
        .unwrap_or_else(|e| {
            tracing::warn!("Failed to load config from {}: {}, using defaults + env", config_path.display(), e);
            ComfyConfig::from_env()
        });

    if !config_path.exists() {
        if let Err(e) = config.save(&config_path) {
            tracing::warn!("Failed to save initial config to {}: {}", config_path.display(), e);
        } else {
            tracing::info!("Created default config at {}", config_path.display());
        }
    }

    let mut registry = NodeRegistry::new();
    builtin_nodes::register_builtin_nodes(&mut registry);

    let state = AppState::with_config(registry, config, config_path.clone());

    let addr: SocketAddr = state.config.read().unwrap().address().parse().unwrap_or_else(|_| {
        tracing::warn!("Invalid address, falling back to 0.0.0.0:8188");
        "0.0.0.0:8188".parse().unwrap()
    });

    let server_config = state.config.read().unwrap().clone();

    let static_dir = server_config.server.static_dir.clone()
        .or_else(|| {
            let default = std::path::Path::new("../../comfy-ui/dist").to_path_buf();
            if default.exists() {
                Some(default.to_string_lossy().to_string())
            } else {
                None
            }
        });

    let mut server = ComfyServer::new(state, addr).with_config(server_config);
    if let Some(ref dir) = static_dir {
        server = server.with_static_dir(dir);
    }

    tracing::info!("Starting ComfyUI-Rust server on {}", addr);
    tracing::info!("Config file: {}", config_path.display());

    if let Err(e) = server.start().await {
        tracing::error!("Server error: {}", e);
        std::process::exit(1);
    }
}

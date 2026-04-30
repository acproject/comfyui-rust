use comfy_api::{AppState, ComfyConfig, ComfyServer};
use comfy_executor::builtin_nodes;
use comfy_executor::NodeRegistry;
use std::net::SocketAddr;
use std::path::Path;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let config_path = std::env::var("COMFY_CONFIG_PATH")
        .unwrap_or_else(|_| "config.json".to_string());
    let config = ComfyConfig::load(Path::new(&config_path))
        .unwrap_or_else(|e| {
            tracing::warn!("Failed to load config from {}: {}, using defaults + env", config_path, e);
            ComfyConfig::from_env()
        });

    let mut registry = NodeRegistry::new();
    builtin_nodes::register_builtin_nodes(&mut registry);

    let state = AppState::with_output_dir(registry, config.output.dir.clone());

    let addr: SocketAddr = config.address().parse().unwrap_or_else(|_| {
        tracing::warn!("Invalid address: {}, falling back to 0.0.0.0:8188", config.address());
        "0.0.0.0:8188".parse().unwrap()
    });

    let static_dir = config.server.static_dir.clone()
        .or_else(|| {
            let default = std::path::Path::new("../../comfy-ui/dist").to_path_buf();
            if default.exists() {
                Some(default.to_string_lossy().to_string())
            } else {
                None
            }
        });

    let mut server = ComfyServer::new(state, addr).with_config(config);
    if let Some(ref dir) = static_dir {
        server = server.with_static_dir(dir);
    }

    tracing::info!("Starting ComfyUI-Rust server on {}", addr);

    if let Err(e) = server.start().await {
        tracing::error!("Server error: {}", e);
        std::process::exit(1);
    }
}

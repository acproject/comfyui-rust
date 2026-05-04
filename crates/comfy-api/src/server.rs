use crate::config::ComfyConfig;
use crate::routes;
use crate::state::AppState;
use crate::worker;
use axum::Router;
use axum::extract::DefaultBodyLimit;
use std::net::SocketAddr;
use tower_http::cors::CorsLayer;
use tower_http::services::ServeDir;

pub struct ComfyServer {
    state: AppState,
    addr: SocketAddr,
    static_dir: Option<String>,
    config: ComfyConfig,
}

impl ComfyServer {
    pub fn new(state: AppState, addr: SocketAddr) -> Self {
        Self {
            state,
            addr,
            static_dir: None,
            config: ComfyConfig::default(),
        }
    }

    pub fn with_config(mut self, config: ComfyConfig) -> Self {
        self.config = config;
        self
    }

    pub fn with_static_dir(mut self, dir: impl Into<String>) -> Self {
        self.static_dir = Some(dir.into());
        self
    }

    pub fn router(&self) -> Router {
        let api_routes = Router::new()
            .route("/prompt", axum::routing::post(routes::post_prompt))
            .route("/prompt", axum::routing::get(routes::get_prompt))
            .route("/queue", axum::routing::post(routes::post_queue))
            .route("/queue", axum::routing::get(routes::get_queue))
            .route("/interrupt", axum::routing::post(routes::post_interrupt))
            .route("/free", axum::routing::post(routes::post_free))
            .route("/history", axum::routing::get(routes::get_history))
            .route("/history", axum::routing::post(routes::post_history))
            .route("/history/{prompt_id}", axum::routing::get(routes::get_history_by_id))
            .route("/object_info", axum::routing::get(routes::get_object_info))
            .route("/object_info/{node_class}", axum::routing::get(routes::get_object_info_by_class))
            .route("/system_stats", axum::routing::get(routes::get_system_stats))
            .route("/embeddings", axum::routing::get(routes::get_embeddings))
            .route("/models", axum::routing::get(routes::get_models))
            .route("/model_manager/list", axum::routing::get(routes::list_model_files))
            .route("/model_manager/delete", axum::routing::post(routes::delete_model_file))
            .route("/extensions", axum::routing::get(routes::get_extensions))
            .route("/ws", axum::routing::get(routes::ws_handler))
            .route("/view", axum::routing::get(routes::get_image))
            .route("/view", axum::routing::post(routes::get_image))
            .route("/view_input", axum::routing::get(routes::get_view_input_image))
            .route("/list_images", axum::routing::get(routes::get_image_list))
            .route("/input_images", axum::routing::get(routes::get_input_images))
            .route("/workflow", axum::routing::post(routes::post_save_workflow))
            .route("/workflow", axum::routing::get(routes::get_load_workflow))
            .route("/workflows", axum::routing::get(routes::get_list_workflows))
            .route("/config", axum::routing::get(routes::get_config))
            .route("/config", axum::routing::post(routes::post_config))
            .route("/custom_nodes", axum::routing::get(routes::list_custom_nodes))
            .route("/custom_nodes", axum::routing::post(routes::save_custom_node))
            .route("/custom_nodes/{filename}", axum::routing::delete(routes::delete_custom_node))
            .route("/agent/config", axum::routing::get(routes::get_agent_config))
            .route("/agent/config", axum::routing::post(routes::post_agent_config))
            .route("/agent/chat", axum::routing::post(routes::post_agent_chat))
            .route("/agent/models", axum::routing::get(routes::get_agent_models))
            .route("/llm/config", axum::routing::get(routes::get_llm_config))
            .route("/llm/config", axum::routing::post(routes::post_llm_config))
            .route("/model_downloads", axum::routing::get(routes::get_model_download_list))
            .route("/model_downloads/download", axum::routing::post(routes::post_download_model))
            .route("/model_downloads/progress", axum::routing::get(routes::get_download_progress))
            .route("/model_downloads/progress", axum::routing::delete(routes::delete_download_progress))
            .with_state(self.state.clone());

        let upload_routes = Router::new()
            .route("/model_manager/upload", axum::routing::post(routes::upload_model_file))
            .route("/upload/image", axum::routing::post(routes::post_upload_image))
            .route("/upload/input_image", axum::routing::post(routes::post_upload_input_image))
            .layer(DefaultBodyLimit::disable())
            .with_state(self.state.clone());

        let mut router = Router::new().merge(api_routes).merge(upload_routes);

        if let Some(ref static_dir) = self.static_dir {
            router = router.fallback_service(ServeDir::new(static_dir));
        }

        router.layer(CorsLayer::permissive())
    }

    pub async fn start(self) -> Result<(), Box<dyn std::error::Error>> {
        let state = self.state.clone();
        let addr = self.addr;
        let app = self.router();

        let executor_state = state.clone();
        let executor_handle = tokio::spawn(async move {
            worker::run_executor(executor_state).await;
        });

        let listener = tokio::net::TcpListener::bind(addr).await?;
        tracing::info!("ComfyUI-Rust server listening on {}", addr);
        tracing::info!("Config: backend={}, models_dir={}, output_dir={}",
            self.config.inference.backend,
            self.config.models.base_dir,
            self.config.output.dir
        );

        if self.static_dir.is_some() {
            tracing::info!("Serving static files from: {:?}", self.static_dir);
        }

        axum::serve(listener, app)
            .with_graceful_shutdown(shutdown_signal())
            .await?;

        state.queue.shutdown().await;
        executor_handle.abort();

        Ok(())
    }
}

async fn shutdown_signal() {
    tokio::signal::ctrl_c()
        .await
        .expect("Failed to install CTRL+C signal handler");
    tracing::info!("Shutdown signal received");
}

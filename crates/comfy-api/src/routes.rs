use crate::agent::{AgentConfig, ChatRequest};
use crate::config::ComfyConfig;
use crate::error::ApiError;
use crate::state::AppState;
use crate::ws::WsMessage;
use axum::body::Body;
use axum::extract::{Path, Query, State, WebSocketUpgrade};
use axum::http::{header, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Json;
use futures::{SinkExt, StreamExt};
use serde::Deserialize;
use serde_json::{json, Value};
use std::collections::HashMap;

#[derive(Deserialize)]
pub struct PromptRequest {
    pub prompt: HashMap<String, Value>,
    #[serde(default)]
    pub extra_data: HashMap<String, Value>,
    #[serde(default)]
    pub client_id: Option<String>,
    #[serde(default)]
    pub prompt_id: Option<String>,
    #[serde(default)]
    pub front: Option<bool>,
}

pub async fn post_prompt(
    State(state): State<AppState>,
    Json(body): Json<PromptRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let prompt = body.prompt;
    let extra_data = body.extra_data;
    let client_id = body.client_id;
    let front = body.front.unwrap_or(false);

    let validation = state.executor.validate_prompt(&prompt);
    if !validation.valid {
        return Err(ApiError::Validation(validation));
    }

    let (prompt_id, number) = state.queue.put(prompt, extra_data, client_id, front).await;

    let queue_info = state.queue.get_queue_info().await;
    state.broadcaster.send(WsMessage::status(queue_info, ""));

    Ok(Json(json!({
        "prompt_id": prompt_id,
        "number": number,
        "node_errors": validation.node_errors,
    })))
}

pub async fn get_prompt(
    State(state): State<AppState>,
) -> Result<impl IntoResponse, ApiError> {
    let queue_info = state.queue.get_queue_info().await;
    Ok(Json(queue_info))
}

#[derive(Deserialize)]
pub struct QueueRequest {
    #[serde(default)]
    pub clear: Option<bool>,
    #[serde(default)]
    pub delete: Option<Vec<String>>,
}

pub async fn post_queue(
    State(state): State<AppState>,
    Json(body): Json<QueueRequest>,
) -> Result<impl IntoResponse, ApiError> {
    if body.clear.unwrap_or(false) {
        state.queue.clear_queue().await;
    }

    if let Some(delete_ids) = body.delete {
        for id in delete_ids {
            state.queue.delete_queue_item(&id).await;
        }
    }

    let queue_info = state.queue.get_queue_info().await;
    state.broadcaster.send(WsMessage::status(queue_info, ""));

    Ok(axum::http::StatusCode::OK)
}

pub async fn get_queue(
    State(state): State<AppState>,
) -> Result<impl IntoResponse, ApiError> {
    let queue_info = state.queue.get_queue_info().await;
    Ok(Json(queue_info))
}

pub async fn post_interrupt(
    State(state): State<AppState>,
) -> Result<impl IntoResponse, ApiError> {
    state.queue.interrupt().await;
    Ok(axum::http::StatusCode::OK)
}

#[derive(Deserialize)]
pub struct FreeRequest {
    #[serde(default)]
    pub unload_models: Option<bool>,
    #[serde(default)]
    pub free_memory: Option<bool>,
}

pub async fn post_free(
    State(_state): State<AppState>,
    Json(_body): Json<FreeRequest>,
) -> Result<impl IntoResponse, ApiError> {
    Ok(axum::http::StatusCode::OK)
}

#[derive(Deserialize)]
pub struct HistoryQuery {
    pub max_items: Option<usize>,
    pub offset: Option<usize>,
}

#[derive(Deserialize)]
pub struct HistoryDeleteRequest {
    #[serde(default)]
    pub clear: Option<bool>,
    #[serde(default)]
    pub delete: Option<Vec<String>>,
}

pub async fn get_history(
    State(state): State<AppState>,
    Query(query): Query<HistoryQuery>,
) -> Result<impl IntoResponse, ApiError> {
    let history = state
        .queue
        .get_history(None, query.max_items, query.offset)
        .await;
    Ok(Json(json!(history)))
}

pub async fn get_history_by_id(
    State(state): State<AppState>,
    Path(prompt_id): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    let history = state.queue.get_history(Some(&prompt_id), None, None).await;
    if history.is_empty() {
        return Err(ApiError::NotFound(format!(
            "History not found for prompt_id: {}",
            prompt_id
        )));
    }
    Ok(Json(json!(history)))
}

pub async fn post_history(
    State(state): State<AppState>,
    Json(body): Json<HistoryDeleteRequest>,
) -> Result<impl IntoResponse, ApiError> {
    if body.clear.unwrap_or(false) {
        state.queue.clear_history().await;
    }

    if let Some(delete_ids) = body.delete {
        for id in delete_ids {
            state.queue.delete_history_item(&id).await;
        }
    }

    Ok(axum::http::StatusCode::OK)
}

pub async fn get_object_info(
    State(state): State<AppState>,
) -> Result<impl IntoResponse, ApiError> {
    let registry = state.registry.lock().await;
    let info = registry.get_all_class_defs();
    let mut result = HashMap::new();

    for (class_type, class_def) in info {
        result.insert(
            class_type.to_string(),
            serde_json::to_value(class_def).unwrap_or(Value::Null),
        );
    }

    Ok(Json(json!(result)))
}

pub async fn get_object_info_by_class(
    State(state): State<AppState>,
    Path(node_class): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    let registry = state.registry.lock().await;
    match registry.get_class_def(&node_class) {
        Some(class_def) => Ok(Json(serde_json::to_value(class_def).unwrap_or(Value::Null))),
        None => Err(ApiError::NotFound(format!(
            "Node class not found: {}",
            node_class
        ))),
    }
}

pub async fn get_system_stats(
    State(_state): State<AppState>,
) -> Result<impl IntoResponse, ApiError> {
    Ok(Json(json!({
        "system": {
            "cpu": "unknown",
            "memory": "unknown",
        },
        "devices": [],
    })))
}

pub async fn get_embeddings(
    State(_state): State<AppState>,
) -> Result<impl IntoResponse, ApiError> {
    Ok(Json(json!([])))
}

pub async fn get_models(
    State(_state): State<AppState>,
) -> Result<impl IntoResponse, ApiError> {
    Ok(Json(json!([])))
}

#[derive(Deserialize)]
pub struct ModelListQuery {
    pub model_type: Option<String>,
}

pub async fn list_model_files(
    State(state): State<AppState>,
    Query(query): Query<ModelListQuery>,
) -> Result<impl IntoResponse, ApiError> {
    let config = state.config.read().map_err(|e| ApiError::Internal(e.to_string()))?;
    let models_dir = &state.models_dir;

    let model_types: Vec<&str> = if let Some(ref mt) = query.model_type {
        vec![mt.as_str()]
    } else {
        ComfyConfig::model_types()
    };

    let mut result = serde_json::Map::new();

    for model_type in model_types {
        let sub_dir = config.get_model_type_dir(model_type);
        let dir_path = models_dir.join(&sub_dir);

        let mut files: Vec<serde_json::Value> = Vec::new();

        if dir_path.exists() {
            scan_model_files(&dir_path, &dir_path, &mut files);
        }

        result.insert(
            model_type.to_string(),
            serde_json::Value::Array(files),
        );
    }

    Ok(Json(serde_json::Value::Object(result)))
}

fn scan_model_files(
    dir: &std::path::Path,
    base: &std::path::Path,
    results: &mut Vec<serde_json::Value>,
) {
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                scan_model_files(&path, base, results);
            } else if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                let lower = name.to_lowercase();
                let is_model = lower.ends_with(".safetensors")
                    || lower.ends_with(".ckpt")
                    || lower.ends_with(".pt")
                    || lower.ends_with(".pth")
                    || lower.ends_with(".bin")
                    || lower.ends_with(".onnx")
                    || lower.ends_with(".gguf")
                    || lower.ends_with(".sft")
                    || lower.ends_with(".json");

                if is_model {
                    if let Ok(rel) = path.strip_prefix(base) {
                        let rel_path = rel.to_string_lossy().to_string();
                        let metadata = std::fs::metadata(&path).ok();
                        let size = metadata.as_ref().map(|m| m.len()).unwrap_or(0);
                        let modified = metadata
                            .and_then(|m| m.modified().ok())
                            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                            .map(|d| d.as_secs());

                        results.push(json!({
                            "name": name,
                            "path": rel_path,
                            "size": size,
                            "modified": modified,
                        }));
                    }
                }
            }
        }
    }
}

#[derive(Deserialize)]
pub struct DeleteModelRequest {
    pub model_type: String,
    pub path: String,
}

pub async fn delete_model_file(
    State(state): State<AppState>,
    Json(body): Json<DeleteModelRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let config = state.config.read().map_err(|e| ApiError::Internal(e.to_string()))?;
    let sub_dir = config.get_model_type_dir(&body.model_type);
    let file_path = state.models_dir.join(&sub_dir).join(&body.path);

    if !file_path.exists() {
        return Err(ApiError::NotFound(format!(
            "Model file not found: {}",
            body.path
        )));
    }

    let canonical = file_path.canonicalize().map_err(|e| ApiError::Internal(e.to_string()))?;
    let models_canonical = state.models_dir.canonicalize().map_err(|e| ApiError::Internal(e.to_string()))?;

    if !canonical.starts_with(&models_canonical) {
        return Err(ApiError::BadRequest("Invalid model path".to_string()));
    }

    std::fs::remove_file(&canonical).map_err(|e| ApiError::Internal(e.to_string()))?;

    Ok(Json(json!({ "success": true, "path": body.path })))
}

pub async fn upload_model_file(
    State(state): State<AppState>,
    mut multipart: axum::extract::Multipart,
) -> Result<impl IntoResponse, ApiError> {
    let mut model_type = String::new();
    let mut filename = String::new();
    let mut data = Vec::new();

    while let Some(field) = multipart.next_field().await.map_err(|e| {
        ApiError::BadRequest(format!("Multipart error: {}", e))
    })? {
        let name = field.name().unwrap_or("").to_string();
        match name.as_str() {
            "file" => {
                filename = field.file_name().unwrap_or("model.safetensors").to_string();
                data = field.bytes().await.map_err(|e| ApiError::BadRequest(format!("Read error: {}", e)))?.to_vec();
            }
            "model_type" => {
                model_type = String::from_utf8(
                    field.bytes().await.map_err(|e| ApiError::BadRequest(format!("Read error: {}", e)))?.to_vec(),
                ).unwrap_or_default();
            }
            _ => {}
        }
    }

    if data.is_empty() {
        return Err(ApiError::BadRequest("No file data provided".to_string()));
    }

    if model_type.is_empty() {
        return Err(ApiError::BadRequest("model_type is required".to_string()));
    }

    let config = state.config.read().map_err(|e| ApiError::Internal(e.to_string()))?;
    let sub_dir = config.get_model_type_dir(&model_type);
    let dir = state.models_dir.join(&sub_dir);
    std::fs::create_dir_all(&dir).map_err(|e| ApiError::Internal(e.to_string()))?;

    let file_path = dir.join(&filename);
    std::fs::write(&file_path, &data).map_err(|e| ApiError::Internal(e.to_string()))?;

    Ok(Json(json!({
        "name": filename,
        "model_type": model_type,
        "path": filename,
    })))
}

pub async fn get_input_images(
    State(state): State<AppState>,
) -> Result<impl IntoResponse, ApiError> {
    let input_dir = state.input_dir.clone();
    let mut images: Vec<String> = Vec::new();

    if input_dir.exists() {
        scan_image_files(&input_dir, &input_dir, &mut images);
    }

    Ok(Json(json!({ "images": images })))
}

fn scan_image_files(dir: &std::path::Path, base: &std::path::Path, results: &mut Vec<String>) {
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                scan_image_files(&path, base, results);
            } else if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                let lower = name.to_lowercase();
                if lower.ends_with(".png") || lower.ends_with(".jpg") || lower.ends_with(".jpeg") || lower.ends_with(".webp") || lower.ends_with(".gif") || lower.ends_with(".bmp") {
                    if let Ok(rel) = path.strip_prefix(base) {
                        results.push(rel.to_string_lossy().to_string());
                    }
                }
            }
        }
    }
}

pub async fn post_upload_input_image(
    State(state): State<AppState>,
    mut multipart: axum::extract::Multipart,
) -> Result<impl IntoResponse, ApiError> {
    let mut filename = String::new();
    let mut subfolder = String::new();
    let mut data = Vec::new();

    while let Some(field) = multipart.next_field().await.map_err(|e| {
        ApiError::BadRequest(format!("Multipart error: {}", e))
    })? {
        let name = field.name().unwrap_or("").to_string();
        match name.as_str() {
            "image" => {
                filename = field.file_name().unwrap_or("upload.png").to_string();
                data = field.bytes().await.map_err(|e| ApiError::BadRequest(format!("Read error: {}", e)))?.to_vec();
            }
            "subfolder" => {
                subfolder = String::from_utf8(
                    field.bytes().await.map_err(|e| ApiError::BadRequest(format!("Read error: {}", e)))?.to_vec(),
                ).unwrap_or_default();
            }
            _ => {}
        }
    }

    if data.is_empty() {
        return Err(ApiError::BadRequest("No image data provided".to_string()));
    }

    let dir = if subfolder.is_empty() {
        state.input_dir.clone()
    } else {
        let dir = state.input_dir.join(&subfolder);
        std::fs::create_dir_all(&dir).map_err(|e| ApiError::Internal(e.to_string()))?;
        dir
    };

    let file_path = dir.join(&filename);
    std::fs::write(&file_path, &data).map_err(|e| ApiError::Internal(e.to_string()))?;

    let name = if subfolder.is_empty() {
        filename.clone()
    } else {
        format!("{}/{}", subfolder, filename)
    };

    Ok(Json(json!({
        "name": name,
        "subfolder": subfolder,
        "type": "input",
    })))
}

pub async fn get_extensions(
    State(_state): State<AppState>,
) -> Result<impl IntoResponse, ApiError> {
    Ok(Json(json!([])))
}

pub async fn ws_handler(
    ws: WebSocketUpgrade,
    Query(params): Query<HashMap<String, String>>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let client_id = params.get("clientId").cloned().unwrap_or_default();
    ws.on_upgrade(move |socket| handle_ws(socket, state, client_id))
}

async fn handle_ws(socket: axum::extract::ws::WebSocket, state: AppState, client_id: String) {
    let (mut sender, mut receiver) = socket.split();

    if !client_id.is_empty() {
        state
            .broadcaster
            .register_client(client_id.clone(), client_id.clone())
            .await;
    }

    let queue_info = state.queue.get_queue_info().await;
    let init_msg = WsMessage::status(queue_info, &client_id);
    if let Ok(text) = serde_json::to_string(&init_msg) {
        let _ = sender.send(axum::extract::ws::Message::Text(text.into())).await;
    }

    let mut rx = state.broadcaster.subscribe();
    let client_id_clone = client_id.clone();
    let state_clone = state.clone();

    let send_task = tokio::spawn(async move {
        while let Ok(msg) = rx.recv().await {
            if let Ok(text) = serde_json::to_string(&msg) {
                if sender
                    .send(axum::extract::ws::Message::Text(text.into()))
                    .await
                    .is_err()
                {
                    break;
                }
            }
        }
    });

    let recv_task = tokio::spawn(async move {
        while let Some(Ok(msg)) = receiver.next().await {
            match msg {
                axum::extract::ws::Message::Text(text) => {
                    if let Ok(data) = serde_json::from_str::<Value>(&text) {
                        let _msg_type = data.get("type").and_then(|v| v.as_str()).unwrap_or("");
                    }
                }
                axum::extract::ws::Message::Close(_) => break,
                _ => {}
            }
        }
    });

    tokio::select! {
        _ = send_task => {},
        _ = recv_task => {},
    }

    if !client_id_clone.is_empty() {
        state_clone
            .broadcaster
            .unregister_client(&client_id_clone)
            .await;
    }
}

#[derive(Deserialize)]
pub struct ImageQuery {
    pub subfolder: Option<String>,
    pub filename: Option<String>,
    pub type_: Option<String>,
}

pub async fn get_image(
    State(state): State<AppState>,
    Query(query): Query<ImageQuery>,
) -> Result<Response, ApiError> {
    let subfolder = query.subfolder.unwrap_or_default();
    let filename = query.filename.unwrap_or_default();

    if filename.is_empty() {
        return Err(ApiError::BadRequest("filename is required".to_string()));
    }

    let image_id = if subfolder.is_empty() {
        filename.clone()
    } else {
        format!("{}/{}", subfolder, filename)
    };

    match state.images.get_image(&image_id).await {
        Some((data, content_type)) => {
            let _cache_control = header::HeaderValue::from_static("public, max-age=31536000");

            Response::builder()
                .status(StatusCode::OK)
                .header(header::CONTENT_TYPE, &content_type)
                .header(header::CACHE_CONTROL, "public, max-age=31536000")
                .body(Body::from(data))
                .map_err(|e| ApiError::Internal(e.to_string()))
        }
        None => Err(ApiError::NotFound(format!("Image not found: {}", image_id))),
    }
}

pub async fn get_view_input_image(
    State(state): State<AppState>,
    Query(query): Query<ImageQuery>,
) -> Result<Response, ApiError> {
    let subfolder = query.subfolder.unwrap_or_default();
    let filename = query.filename.unwrap_or_default();

    if filename.is_empty() {
        return Err(ApiError::BadRequest("filename is required".to_string()));
    }

    let path = if subfolder.is_empty() {
        state.input_dir.join(&filename)
    } else {
        state.input_dir.join(&subfolder).join(&filename)
    };

    if !path.exists() {
        return Err(ApiError::NotFound(format!("Image not found: {}", filename)));
    }

    let data = std::fs::read(&path).map_err(|e| ApiError::Internal(e.to_string()))?;
    let content_type = guess_content_type_from_path(&path);

    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, &content_type)
        .header(header::CACHE_CONTROL, "public, max-age=31536000")
        .body(Body::from(data))
        .map_err(|e| ApiError::Internal(e.to_string()))
}

fn guess_content_type_from_path(path: &std::path::Path) -> String {
    match path.extension().and_then(|e| e.to_str()).unwrap_or("") {
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "webp" => "image/webp",
        "gif" => "image/gif",
        "bmp" => "image/bmp",
        _ => "application/octet-stream",
    }.to_string()
}

pub async fn get_image_list(
    State(state): State<AppState>,
    Query(query): Query<ImageQuery>,
) -> Result<impl IntoResponse, ApiError> {
    let subfolder = query.subfolder.as_deref();
    let images = state.images.list_images(subfolder).await;

    let entries: Vec<Value> = images
        .iter()
        .map(|entry| {
            json!({
                "filename": entry.filename,
                "subfolder": entry.subfolder,
                "type": "output",
            })
        })
        .collect();

    Ok(Json(json!({
        "images": entries,
    })))
}

pub async fn post_upload_image(
    State(state): State<AppState>,
    mut multipart: axum::extract::Multipart,
) -> Result<impl IntoResponse, ApiError> {
    let mut filename = String::new();
    let mut subfolder = String::new();
    let mut data = Vec::new();

    while let Some(field) = multipart.next_field().await.map_err(|e| {
        ApiError::BadRequest(format!("Multipart error: {}", e))
    })? {
        let name = field.name().unwrap_or("").to_string();
        match name.as_str() {
            "image" => {
                filename = field
                    .file_name()
                    .unwrap_or("upload.png")
                    .to_string();
                data = field
                    .bytes()
                    .await
                    .map_err(|e| ApiError::BadRequest(format!("Read error: {}", e)))?
                    .to_vec();
            }
            "subfolder" => {
                subfolder = String::from_utf8(
                    field
                        .bytes()
                        .await
                        .map_err(|e| ApiError::BadRequest(format!("Read error: {}", e)))?
                        .to_vec(),
                )
                .unwrap_or_default();
            }
            _ => {}
        }
    }

    if data.is_empty() {
        return Err(ApiError::BadRequest("No image data provided".to_string()));
    }

    let _image_id = state
        .images
        .save_image(&data, &filename, &subfolder, None)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    Ok(Json(json!({
        "name": filename,
        "subfolder": subfolder,
        "type": "output",
    })))
}

#[derive(Deserialize)]
pub struct SaveWorkflowRequest {
    pub name: String,
    pub workflow: serde_json::Value,
    #[serde(default)]
    pub description: Option<String>,
}

pub async fn post_save_workflow(
    State(state): State<AppState>,
    Json(body): Json<SaveWorkflowRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let output_dir = state.images.output_dir();
    let workflows_dir = output_dir.join("workflows");
    std::fs::create_dir_all(&workflows_dir).map_err(|e| ApiError::Internal(e.to_string()))?;

    let filename = format!("{}.json", body.name);
    let path = workflows_dir.join(&filename);

    let workflow_data = serde_json::to_string_pretty(&body.workflow)
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    std::fs::write(&path, workflow_data).map_err(|e| ApiError::Internal(e.to_string()))?;

    Ok(Json(json!({
        "name": body.name,
        "path": filename,
    })))
}

#[derive(Deserialize)]
pub struct LoadWorkflowQuery {
    pub name: String,
}

pub async fn get_load_workflow(
    State(state): State<AppState>,
    Query(query): Query<LoadWorkflowQuery>,
) -> Result<impl IntoResponse, ApiError> {
    let output_dir = state.images.output_dir();
    let workflows_dir = output_dir.join("workflows");
    let filename = format!("{}.json", query.name);
    let path = workflows_dir.join(&filename);

    if !path.exists() {
        return Err(ApiError::NotFound(format!(
            "Workflow not found: {}",
            query.name
        )));
    }

    let data = std::fs::read_to_string(&path).map_err(|e| ApiError::Internal(e.to_string()))?;

    let workflow: serde_json::Value =
        serde_json::from_str(&data).map_err(|e| ApiError::Internal(e.to_string()))?;

    Ok(Json(workflow))
}

pub async fn get_list_workflows(
    State(state): State<AppState>,
) -> Result<impl IntoResponse, ApiError> {
    let output_dir = state.images.output_dir();
    let workflows_dir = output_dir.join("workflows");

    if !workflows_dir.exists() {
        return Ok(Json(json!({ "workflows": [] })));
    }

    let mut workflows = Vec::new();
    let entries = std::fs::read_dir(&workflows_dir).map_err(|e| ApiError::Internal(e.to_string()))?;

    for entry in entries {
        let entry = entry.map_err(|e| ApiError::Internal(e.to_string()))?;
        let path = entry.path();
        if path.extension().map(|e| e == "json").unwrap_or(false) {
            if let Some(name) = path.file_stem().and_then(|s| s.to_str()) {
                let metadata = entry.metadata().map_err(|e| ApiError::Internal(e.to_string()))?;
                workflows.push(json!({
                    "name": name,
                    "size": metadata.len(),
                    "modified": metadata.modified().ok().map(|t| {
                        std::time::SystemTime::from(t)
                            .duration_since(std::time::UNIX_EPOCH)
                            .ok()
                            .map(|d| d.as_secs())
                    }),
                }));
            }
        }
    }

    Ok(Json(json!({ "workflows": workflows })))
}

pub async fn get_config(
    State(state): State<AppState>,
) -> Result<impl IntoResponse, ApiError> {
    let config = state.config.read().map_err(|e| ApiError::Internal(e.to_string()))?;
    Ok(Json(serde_json::to_value(&*config).unwrap_or(Value::Null)))
}

pub async fn post_config(
    State(state): State<AppState>,
    Json(body): Json<ComfyConfig>,
) -> Result<impl IntoResponse, ApiError> {
    {
        let mut config = state.config.write().map_err(|e| ApiError::Internal(e.to_string()))?;
        *config = body.clone();
    }
    let config = state.config.read().map_err(|e| ApiError::Internal(e.to_string()))?;
    config.save(&state.config_path)
        .map_err(|e| ApiError::Internal(format!("Failed to save config: {}", e)))?;

    Ok(Json(serde_json::to_value(&*config).unwrap_or(Value::Null)))
}

pub async fn list_custom_nodes(
    State(state): State<AppState>,
) -> Result<impl IntoResponse, ApiError> {
    let custom_nodes_dir = &state.custom_nodes_dir;
    if !custom_nodes_dir.exists() {
        return Ok(Json(json!({ "nodes": [] })));
    }

    let mut nodes = Vec::new();
    let entries = std::fs::read_dir(custom_nodes_dir).map_err(|e| ApiError::Internal(e.to_string()))?;

    for entry in entries {
        let entry = entry.map_err(|e| ApiError::Internal(e.to_string()))?;
        let path = entry.path();
        if path.extension().map(|e| e == "json").unwrap_or(false) {
            let content = std::fs::read_to_string(&path).map_err(|e| ApiError::Internal(e.to_string()))?;
            if let Ok(node_value) = serde_json::from_str::<Value>(&content) {
                let filename = path.file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("unknown")
                    .to_string();
                nodes.push(json!({
                    "filename": filename,
                    "definition": node_value,
                }));
            }
        }
    }

    Ok(Json(json!({ "nodes": nodes })))
}

#[derive(Deserialize)]
pub struct SaveCustomNodeRequest {
    pub filename: String,
    pub definition: serde_json::Value,
}

pub async fn save_custom_node(
    State(state): State<AppState>,
    Json(body): Json<SaveCustomNodeRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let custom_nodes_dir = &state.custom_nodes_dir;
    std::fs::create_dir_all(custom_nodes_dir).map_err(|e| ApiError::Internal(e.to_string()))?;

    let safe_filename = body.filename
        .replace("..", "")
        .replace("/", "")
        .replace("\\", "");
    if safe_filename.is_empty() {
        return Err(ApiError::BadRequest("Invalid filename".to_string()));
    }

    let filename = format!("{}.json", safe_filename);
    let path = custom_nodes_dir.join(&filename);

    let content = serde_json::to_string_pretty(&body.definition)
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    std::fs::write(&path, content).map_err(|e| ApiError::Internal(e.to_string()))?;

    Ok(Json(json!({
        "filename": safe_filename,
        "path": filename,
    })))
}

pub async fn delete_custom_node(
    State(state): State<AppState>,
    Path(filename): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    let safe_filename = filename
        .replace("..", "")
        .replace("/", "")
        .replace("\\", "");
    if safe_filename.is_empty() {
        return Err(ApiError::BadRequest("Invalid filename".to_string()));
    }

    let path = state.custom_nodes_dir.join(format!("{}.json", safe_filename));

    if !path.exists() {
        return Err(ApiError::NotFound(format!("Custom node not found: {}", safe_filename)));
    }

    std::fs::remove_file(&path).map_err(|e| ApiError::Internal(e.to_string()))?;

    Ok(axum::http::StatusCode::OK)
}

pub async fn get_agent_config(
    State(state): State<AppState>,
) -> Result<impl IntoResponse, ApiError> {
    let config = state.agent.get_config().await;
    let safe_config = AgentConfig {
        api_key: config.api_key.as_ref().map(|_| "********".to_string()),
        ..config
    };
    Ok(Json(serde_json::to_value(safe_config).unwrap_or(Value::Null)))
}

pub async fn post_agent_config(
    State(state): State<AppState>,
    Json(mut body): Json<AgentConfig>,
) -> Result<impl IntoResponse, ApiError> {
    if body.api_key.as_deref() == Some("********") {
        let existing = state.agent.get_config().await;
        body.api_key = existing.api_key;
    }
    state.agent.set_config(body).await;
    let config = state.agent.get_config().await;
    let safe_config = AgentConfig {
        api_key: config.api_key.as_ref().map(|_| "********".to_string()),
        ..config
    };
    Ok(Json(serde_json::to_value(safe_config).unwrap_or(Value::Null)))
}

pub async fn post_agent_chat(
    State(state): State<AppState>,
    Json(body): Json<ChatRequest>,
) -> Result<impl IntoResponse, ApiError> {
    match state.agent.chat(body).await {
        Ok(response) => Ok(Json(serde_json::to_value(response).unwrap_or(Value::Null))),
        Err(e) => Err(ApiError::Internal(e.to_string())),
    }
}

pub async fn get_agent_models(
    State(state): State<AppState>,
) -> Result<impl IntoResponse, ApiError> {
    match state.agent.list_models().await {
        Ok(models) => Ok(Json(json!({ "models": models }))),
        Err(e) => Err(ApiError::Internal(e.to_string())),
    }
}

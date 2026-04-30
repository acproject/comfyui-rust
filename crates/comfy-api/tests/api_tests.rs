use comfy_api::{AppState, ComfyServer, ComfyConfig, ImageStore};
use comfy_executor::{builtin_nodes, NodeRegistry};
use std::collections::HashMap;
use tower::util::ServiceExt;

fn create_test_app() -> axum::Router {
    let mut registry = NodeRegistry::new();
    builtin_nodes::register_builtin_nodes(&mut registry);
    let temp_dir = std::env::temp_dir().join("comfy_test_output");
    let state = AppState::with_output_dir(registry, temp_dir.to_string_lossy().to_string());
    ComfyServer::new(state, "0.0.0.0:0".parse().unwrap()).router()
}

#[tokio::test]
async fn test_get_image_not_found() {
    let app = create_test_app();
    let response = app
        .oneshot(
            axum::http::Request::builder()
                .uri("/view?filename=nonexistent.png")
                .body(axum::body::Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), axum::http::StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_get_image_missing_filename() {
    let app = create_test_app();
    let response = app
        .oneshot(
            axum::http::Request::builder()
                .uri("/view")
                .body(axum::body::Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), axum::http::StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_list_images_empty() {
    let app = create_test_app();
    let response = app
        .oneshot(
            axum::http::Request::builder()
                .uri("/list_images")
                .body(axum::body::Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), axum::http::StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), 1024 * 1024)
        .await
        .unwrap();
    let data: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert!(data["images"].is_array());
}

#[tokio::test]
async fn test_get_config() {
    let app = create_test_app();
    let response = app
        .oneshot(
            axum::http::Request::builder()
                .uri("/config")
                .body(axum::body::Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), axum::http::StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), 1024 * 1024)
        .await
        .unwrap();
    let data: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert!(data["server"].is_object());
    assert!(data["models"].is_object());
    assert!(data["inference"].is_object());
    assert!(data["output"].is_object());
}

#[tokio::test]
async fn test_list_workflows_empty() {
    let app = create_test_app();
    let response = app
        .oneshot(
            axum::http::Request::builder()
                .uri("/workflows")
                .body(axum::body::Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), axum::http::StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), 1024 * 1024)
        .await
        .unwrap();
    let data: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert!(data["workflows"].is_array());
}

#[tokio::test]
async fn test_save_and_load_workflow() {
    let app = create_test_app();

    let workflow = serde_json::json!({
        "last_node_id": 3,
        "last_link_id": 3,
        "nodes": [],
        "links": [],
        "groups": [],
        "config": {},
        "extra": {},
        "version": 0.4
    });

    let save_body = serde_json::json!({
        "name": "test_workflow",
        "workflow": workflow
    });

    let response = app
        .oneshot(
            axum::http::Request::builder()
                .method("POST")
                .uri("/workflow")
                .header("Content-Type", "application/json")
                .body(axum::body::Body::from(serde_json::to_vec(&save_body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), axum::http::StatusCode::OK);
}

#[tokio::test]
async fn test_load_workflow_not_found() {
    let app = create_test_app();
    let response = app
        .oneshot(
            axum::http::Request::builder()
                .uri("/workflow?name=nonexistent_workflow")
                .body(axum::body::Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), axum::http::StatusCode::NOT_FOUND);
}

#[test]
fn test_image_store_save_and_get() {
    let temp_dir = std::env::temp_dir().join("comfy_test_images");
    let store = ImageStore::new(&temp_dir);

    let test_data = vec![0u8; 100];
    let rt = tokio::runtime::Runtime::new().unwrap();

    let image_id = rt.block_on(async {
        store
            .save_image(&test_data, "test.png", "", None)
            .await
            .unwrap()
    });

    let result = rt.block_on(async { store.get_image(&image_id).await });
    assert!(result.is_some());
    let (data, content_type) = result.unwrap();
    assert_eq!(data, test_data);
    assert_eq!(content_type, "image/png");

    std::fs::remove_dir_all(temp_dir).ok();
}

#[test]
fn test_image_store_with_subfolder() {
    let temp_dir = std::env::temp_dir().join("comfy_test_images_sub");
    let store = ImageStore::new(&temp_dir);

    let test_data = vec![1u8; 50];
    let rt = tokio::runtime::Runtime::new().unwrap();

    let image_id = rt.block_on(async {
        store
            .save_image(&test_data, "sub_test.png", "subfolder1", None)
            .await
            .unwrap()
    });

    assert!(image_id.contains("subfolder1"));

    let result = rt.block_on(async { store.get_image(&image_id).await });
    assert!(result.is_some());

    std::fs::remove_dir_all(temp_dir).ok();
}

#[test]
fn test_image_store_list_and_delete() {
    let temp_dir = std::env::temp_dir().join("comfy_test_images_list");
    let store = ImageStore::new(&temp_dir);

    let rt = tokio::runtime::Runtime::new().unwrap();

    rt.block_on(async {
        store
            .save_image(&[0u8; 10], "img1.png", "", None)
            .await
            .unwrap();
        store
            .save_image(&[1u8; 10], "img2.png", "sub", None)
            .await
            .unwrap();
    });

    let all = rt.block_on(async { store.list_images(None).await });
    assert_eq!(all.len(), 2);

    let sub_only = rt.block_on(async { store.list_images(Some("sub")).await });
    assert_eq!(sub_only.len(), 1);

    let image_id_to_delete = if all[0].subfolder.is_empty() {
        all[0].filename.clone()
    } else {
        format!("{}/{}", all[0].subfolder, all[0].filename)
    };
    let deleted = rt.block_on(async { store.delete_image(&image_id_to_delete).await });
    assert!(deleted);

    std::fs::remove_dir_all(temp_dir).ok();
}

#[test]
fn test_config_default() {
    let config = ComfyConfig::default();
    assert_eq!(config.server.host, "127.0.0.1");
    assert_eq!(config.server.port, 8188);
    assert_eq!(config.inference.backend, "local");
    assert_eq!(config.output.format, "png");
}

#[test]
fn test_config_save_and_load() {
    let temp_dir = std::env::temp_dir().join("comfy_test_config");
    let path = temp_dir.join("config.json");

    let config = ComfyConfig::default();
    config.save(&path).unwrap();

    let loaded = ComfyConfig::load(&path).unwrap();
    assert_eq!(loaded.server.host, config.server.host);
    assert_eq!(loaded.server.port, config.server.port);
    assert_eq!(loaded.inference.backend, config.inference.backend);

    std::fs::remove_dir_all(temp_dir).ok();
}

#[test]
fn test_config_from_env() {
    std::env::set_var("COMFY_HOST", "0.0.0.0");
    std::env::set_var("COMFY_PORT", "9999");
    std::env::set_var("COMFY_BACKEND", "remote");

    let config = ComfyConfig::from_env();
    assert_eq!(config.server.host, "0.0.0.0");
    assert_eq!(config.server.port, 9999);
    assert_eq!(config.inference.backend, "remote");

    std::env::remove_var("COMFY_HOST");
    std::env::remove_var("COMFY_PORT");
    std::env::remove_var("COMFY_BACKEND");
}

#[test]
fn test_config_resolve_model_path() {
    let config = ComfyConfig::default();
    let path = config.resolve_model_path("checkpoints", "model.safetensors");
    assert!(path.to_string_lossy().contains("checkpoints"));
    assert!(path.to_string_lossy().contains("model.safetensors"));

    let path = config.resolve_model_path("loras", "lora.safetensors");
    assert!(path.to_string_lossy().contains("loras"));
}

#[test]
fn test_config_address() {
    let config = ComfyConfig::default();
    assert_eq!(config.address(), "127.0.0.1:8188");
}

#[test]
fn test_workflow_serialization() {
    let workflow = comfy_core::Workflow::from_json(r#"{
        "last_node_id": 1,
        "last_link_id": 0,
        "nodes": [{
            "id": 1,
            "type": "SaveImage",
            "pos": [0, 0],
            "size": [200, 100],
            "flags": {"collapsed": false},
            "order": 0,
            "mode": 0,
            "inputs": [],
            "outputs": [],
            "properties": {},
            "widgets_values": []
        }],
        "links": [],
        "groups": [],
        "config": {},
        "extra": {},
        "version": 0.4
    }"#).unwrap();

    assert_eq!(workflow.nodes.len(), 1);
    assert_eq!(workflow.nodes[0].node_type, "SaveImage");

    let json = workflow.to_json().unwrap();
    let reparsed = comfy_core::Workflow::from_json(&json).unwrap();
    assert_eq!(reparsed.nodes.len(), 1);
}

#[test]
fn test_workflow_api_prompt_conversion() {
    let mut prompt = HashMap::new();
    let mut inputs = HashMap::new();
    inputs.insert("ckpt_name".to_string(), serde_json::json!("model.safetensors"));

    prompt.insert(
        "1".to_string(),
        comfy_core::ApiPromptNode {
            class_type: "CheckpointLoaderSimple".to_string(),
            inputs,
        },
    );

    let api_prompt = comfy_core::ApiPrompt {
        prompt,
        extra_data: None,
        client_id: None,
    };

    let workflow = comfy_core::Workflow::from_api_prompt(&api_prompt);
    assert_eq!(workflow.nodes.len(), 1);
    assert_eq!(workflow.nodes[0].node_type, "CheckpointLoaderSimple");

    let back = workflow.to_api_prompt();
    assert_eq!(back.prompt.len(), 1);
    assert!(back.prompt.contains_key("1"));
}

#[tokio::test]
async fn test_full_prompt_workflow_e2e() {
    let app = create_test_app();

    let body = serde_json::json!({
        "prompt": {
            "1": {
                "class_type": "SaveImage",
                "inputs": {
                    "images": ["2", 0],
                    "filename_prefix": "test"
                }
            },
            "2": {
                "class_type": "KSampler",
                "inputs": {
                    "seed": 42,
                    "steps": 20,
                    "cfg": 7.0,
                    "sampler_name": "euler_ancestral",
                    "scheduler": "normal",
                    "model": ["3", 0],
                    "positive": ["4", 0],
                    "negative": ["5", 0],
                    "latent_image": ["6", 0],
                }
            },
            "3": {
                "class_type": "CheckpointLoaderSimple",
                "inputs": {
                    "ckpt_name": "model.safetensors"
                }
            },
            "4": {
                "class_type": "CLIPTextEncode",
                "inputs": {
                    "text": "a beautiful sunset",
                    "clip": ["3", 1]
                }
            },
            "5": {
                "class_type": "CLIPTextEncode",
                "inputs": {
                    "text": "ugly",
                    "clip": ["3", 1]
                }
            },
            "6": {
                "class_type": "EmptyLatentImage",
                "inputs": {
                    "width": 512,
                    "height": 512,
                    "batch_size": 1
                }
            }
        },
        "client_id": "test-client"
    });

    let response = app
        .oneshot(
            axum::http::Request::builder()
                .method("POST")
                .uri("/prompt")
                .header("Content-Type", "application/json")
                .body(axum::body::Body::from(serde_json::to_vec(&body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), axum::http::StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), 1024 * 1024)
        .await
        .unwrap();
    let data: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert!(data["prompt_id"].is_string());
    assert!(data["number"].is_number());
}

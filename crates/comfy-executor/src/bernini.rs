use crate::error::ExecutorError;
use crate::registry::NodeRegistry;
use comfy_core::{IoType, NodeClassDef, NodeInputTypes, InputTypeSpec};
use comfy_inference::image::SdImage;
use comfy_inference::params::{ImageGenParams, ModelConfig};
use serde_json::json;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// Get the models base directory.
fn get_models_dir() -> String {
    std::env::var("COMFY_MODELS_DIR").unwrap_or_else(|_| "models".to_string())
}

/// A Bernini-R directory is usable when it contains the self-contained
/// diffusers layout required by bernini_generate.py (Wan base components plus
/// the two Bernini-R transformer experts).
fn is_self_contained_bernini_dir(dir: &Path) -> bool {
    let name = dir
        .file_name()
        .map(|n| n.to_string_lossy().to_lowercase())
        .unwrap_or_default();
    if !name.contains("bernini") || !dir.is_dir() {
        return false;
    }
    // 14B ships two experts (transformer/ + transformer_2/); the 1.3B variant
    // has a single expert (config skip_transformer_2=true, switch boundary 0).
    dir.join("config.json").exists()
        && dir.join("transformer").is_dir()
        && dir.join("vae").is_dir()
        && dir.join("tokenizer").is_dir()
        && dir.join("text_encoder").is_dir()
}

/// Scan known model locations for self-contained Bernini-R directories.
///
/// Returns `(combo_label, absolute_path)` pairs.
fn scan_bernini_dirs() -> Vec<(String, PathBuf)> {
    let base = PathBuf::from(get_models_dir());
    let mut roots = vec![base.clone(), base.join("diffusion_models")];
    if let Ok(cwd) = std::env::current_dir() {
        roots.push(cwd.join(&base));
    }
    let mut found: Vec<(String, PathBuf)> = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for root in roots {
        let Ok(entries) = fs::read_dir(&root) else { continue };
        for entry in entries.flatten() {
            let path = entry.path();
            if is_self_contained_bernini_dir(&path) {
                let key = path.to_string_lossy().to_string();
                if seen.insert(key.clone()) {
                    found.push((path.file_name().unwrap().to_string_lossy().to_string(), path));
                }
            }
        }
    }
    found.sort_by(|a, b| a.0.cmp(&b.0));
    found
}

/// Resolve a combo label back to a model directory.
fn resolve_bernini_dir(label: &str) -> Option<PathBuf> {
    scan_bernini_dirs()
        .into_iter()
        .find(|(name, _)| name == label)
        .map(|(_, path)| path)
}

fn register_bernini_pipeline(registry: &mut NodeRegistry) {
    let model_choices: Vec<String> = scan_bernini_dirs().into_iter().map(|(n, _)| n).collect();
    let default_model = model_choices.first().cloned().unwrap_or_default();

    let class_def = NodeClassDef {
        class_type: "BerniniRPipeline".to_string(),
        display_name: "Bernini-R 图像生成/编辑".to_string(),
        category: "image/bernini".to_string(),
        input_types: NodeInputTypes {
            required: {
                let mut m = HashMap::new();
                m.insert("prompt".to_string(), InputTypeSpec {
                    type_name: "STRING".to_string(),
                    extra: {
                        let mut e = HashMap::new();
                        e.insert("multiline".to_string(), json!(true));
                        e.insert("default".to_string(), json!(""));
                        e
                    },
                });
                m.insert("bernini_model".to_string(), InputTypeSpec {
                    type_name: "COMBO".to_string(),
                    extra: {
                        let mut e = HashMap::new();
                        e.insert("choices".to_string(),
                            serde_json::Value::Array(model_choices.iter().map(|s| json!(s)).collect()));
                        e
                    },
                });
                m
            },
            optional: {
                let mut m = HashMap::new();
                m.insert("image".to_string(), InputTypeSpec {
                    type_name: "IMAGE".to_string(),
                    extra: HashMap::new(),
                });
                m.insert("negative_prompt".to_string(), InputTypeSpec {
                    type_name: "STRING".to_string(),
                    extra: {
                        let mut e = HashMap::new();
                        e.insert("multiline".to_string(), json!(true));
                        e.insert("default".to_string(), json!(""));
                        e
                    },
                });
                m.insert("seed".to_string(), InputTypeSpec {
                    type_name: "INT".to_string(),
                    extra: {
                        let mut e = HashMap::new();
                        e.insert("min".to_string(), json!(0));
                        e.insert("max".to_string(), json!(u32::MAX as i64));
                        e.insert("step".to_string(), json!(1));
                        e.insert("default".to_string(), json!(42));
                        e
                    },
                });
                m.insert("steps".to_string(), InputTypeSpec {
                    type_name: "INT".to_string(),
                    extra: {
                        let mut e = HashMap::new();
                        e.insert("min".to_string(), json!(1));
                        e.insert("max".to_string(), json!(100));
                        e.insert("step".to_string(), json!(1));
                        e.insert("default".to_string(), json!(40));
                        e
                    },
                });
                m.insert("width".to_string(), InputTypeSpec {
                    type_name: "INT".to_string(),
                    extra: {
                        let mut e = HashMap::new();
                        e.insert("min".to_string(), json!(256));
                        e.insert("max".to_string(), json!(2048));
                        e.insert("step".to_string(), json!(16));
                        e.insert("default".to_string(), json!(848));
                        e
                    },
                });
                m.insert("height".to_string(), InputTypeSpec {
                    type_name: "INT".to_string(),
                    extra: {
                        let mut e = HashMap::new();
                        e.insert("min".to_string(), json!(256));
                        e.insert("max".to_string(), json!(2048));
                        e.insert("step".to_string(), json!(16));
                        e.insert("default".to_string(), json!(480));
                        e
                    },
                });
                m
            },
            hidden: HashMap::new(),
        },
        output_types: vec![IoType::Image],
        output_names: vec!["IMAGE".to_string()],
        output_is_list: vec![false],
        is_output_node: false,
        has_intermediate_output: false,
        is_changed: None,
        not_idempotent: false,
        function_name: "run".to_string(),
    };

    registry.register(class_def, Arc::new(move |ctx, node, node_id| {
        let image_in = ctx.resolve_input(node_id, "image").unwrap_or_else(|_| json!(null));
        let model_label = node.inputs.get("bernini_model")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .unwrap_or_else(|| default_model.clone());
        let prompt = node.inputs.get("prompt")
            .and_then(|v| v.as_str())
            .unwrap_or("").to_string();
        let negative_prompt = node.inputs.get("negative_prompt")
            .and_then(|v| v.as_str())
            .unwrap_or("").to_string();
        let seed = node.inputs.get("seed").and_then(|v| v.as_i64()).unwrap_or(42);
        let steps = node.inputs.get("steps").and_then(|v| v.as_i64()).unwrap_or(40) as i32;
        let width = node.inputs.get("width").and_then(|v| v.as_i64()).unwrap_or(848) as i32;
        let height = node.inputs.get("height").and_then(|v| v.as_i64()).unwrap_or(480) as i32;
        let node_id_str = node_id.to_string();

        Box::pin(async move {
            if prompt.trim().is_empty() {
                return Err(ExecutorError::NodeExecutionFailed {
                    node_id: node_id_str.clone(),
                    message: "BerniniRPipeline requires a prompt".to_string(),
                });
            }

            let model_dir = resolve_bernini_dir(&model_label).ok_or_else(|| {
                ExecutorError::NodeExecutionFailed {
                    node_id: node_id_str.clone(),
                    message: format!(
                        "Bernini-R model directory '{}' not found under COMFY_MODELS_DIR \
                         (need a self-contained *Bernini*-Diffusers directory)",
                        model_label
                    ),
                }
            })?;

            // Optional source image -> i2i. Accept LoadImage's {"path": ...}
            // output as well as an inline serialized SdImage.
            let init_image: Option<SdImage> = if image_in.is_null() {
                None
            } else if let Some(path) = image_in.get("path").and_then(|v| v.as_str()) {
                if path.is_empty() {
                    None
                } else {
                    let resolved = if Path::new(path).is_absolute() {
                        path.to_string()
                    } else if path.starts_with("input/") || Path::new(path).exists() {
                        path.to_string()
                    } else {
                        let input_path = format!("input/{}", path);
                        if Path::new(&input_path).exists() { input_path } else { path.to_string() }
                    };
                    let bytes = fs::read(&resolved).map_err(|e| ExecutorError::NodeExecutionFailed {
                        node_id: node_id_str.clone(),
                        message: format!("Failed to read image file '{}': {}", resolved, e),
                    })?;
                    Some(SdImage::from_png_bytes(&bytes).map_err(|e| ExecutorError::NodeExecutionFailed {
                        node_id: node_id_str.clone(),
                        message: format!("Failed to decode image '{}': {}", resolved, e),
                    })?)
                }
            } else {
                serde_json::from_value::<SdImage>(image_in.clone()).ok()
            };

            let mut params = ImageGenParams::new(prompt)
                .with_seed(seed)
                .with_sample_steps(steps)
                .with_dimensions(width, height)
                .with_model_config(ModelConfig::new().with_model(
                    model_dir.to_string_lossy().to_string()
                ));
            if !negative_prompt.is_empty() {
                params = params.with_negative_prompt(negative_prompt);
            }
            if let Some(img) = init_image {
                params = params.with_init_image(img);
            }

            let backend = ctx.backend();
            let images = backend.generate_image(params)
                .map_err(ExecutorError::Inference)?;

            let image_data: Vec<serde_json::Value> = images.iter()
                .map(|img| serde_json::to_value(img).unwrap_or_else(|_| json!({
                    "type": "image",
                    "width": img.width,
                    "height": img.height,
                    "channel": img.channel,
                })))
                .collect();
            Ok(vec![json!({
                "type": "image",
                "images": image_data,
            })])
        })
    }));
}

pub fn register_bernini_nodes(registry: &mut NodeRegistry) {
    register_bernini_pipeline(registry);
}

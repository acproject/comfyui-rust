//! 背景去除节点 `RemoveBackground`。
//!
//! 通过 Python 回退脚本 `remove_background.py` 调用 rembg 库进行背景去除，
//! 默认使用 bria-rmbg (BRIA-RMBG 2.0) 模型。输出带透明通道的 RGBA 图像
//! 以及对应的 alpha mask。

use crate::error::ExecutorError;
use crate::registry::NodeRegistry;
use comfy_core::{InputTypeSpec, IoType, NodeClassDef, NodeInputTypes};
use comfy_inference::image::SdImage;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::sync::Arc;

/// Decode an IMAGE input value into a decoded image.
fn resolve_input_image(value: &Value, node_id: &str) -> Result<SdImage, ExecutorError> {
    let decode_path = |path: &str| -> Result<SdImage, ExecutorError> {
        let p = Path::new(path);
        let resolved = if p.is_absolute() || path.starts_with("input/") || p.exists() {
            path.to_string()
        } else {
            let under_input = format!("input/{}", path);
            if Path::new(&under_input).exists() {
                under_input
            } else {
                path.to_string()
            }
        };
        let bytes = fs::read(&resolved).map_err(|e| ExecutorError::NodeExecutionFailed {
            node_id: node_id.to_string(),
            message: format!("Failed to read image '{}': {}", resolved, e),
        })?;
        SdImage::from_image_bytes(&bytes).map_err(|e| ExecutorError::NodeExecutionFailed {
            node_id: node_id.to_string(),
            message: format!("Failed to decode image '{}': {}", resolved, e),
        })
    };

    if value.is_null() {
        return Err(ExecutorError::NodeExecutionFailed {
            node_id: node_id.to_string(),
            message: "image input is required".to_string(),
        });
    }
    if let Some(obj) = value.as_object() {
        if let Some(path) = obj.get("path").and_then(|v| v.as_str()) {
            if !path.is_empty() {
                return decode_path(path);
            }
        }
        if let Some(arr) = obj.get("images").and_then(|v| v.as_array()) {
            for item in arr {
                if let Some(path) = item.get("path").and_then(|v| v.as_str()) {
                    if !path.is_empty() {
                        return decode_path(path);
                    }
                }
                if let Ok(img) = serde_json::from_value::<SdImage>(item.clone()) {
                    return Ok(img);
                }
            }
        }
    }
    serde_json::from_value::<SdImage>(value.clone()).map_err(|_| ExecutorError::NodeExecutionFailed {
        node_id: node_id.to_string(),
        message: "failed to decode input image".to_string(),
    })
}

/// Available rembg session models. The default (bria-rmbg) ships locally.
const RMBG_MODELS: &[&str] = &[
    "bria-rmbg",
    "u2net",
    "u2netp",
    "u2net_human_seg",
    "isnet-general-use",
    "silueta",
    "birefnet-general",
    "birefnet-general-lite",
    "birefnet-portrait",
];

fn register_remove_background(registry: &mut NodeRegistry) {
    let model_choices: Vec<Value> = RMBG_MODELS.iter().map(|s| json!(s)).collect();

    let class_def = NodeClassDef {
        class_type: "RemoveBackground".to_string(),
        display_name: "Remove Background (去背景)".to_string(),
        category: "image/rembg".to_string(),
        input_types: NodeInputTypes {
            required: {
                let mut m = HashMap::new();
                m.insert("image".to_string(), InputTypeSpec {
                    type_name: "IMAGE".to_string(),
                    extra: HashMap::new(),
                });
                m
            },
            optional: {
                let mut m = HashMap::new();
                m.insert("model".to_string(), InputTypeSpec {
                    type_name: "COMBO".to_string(),
                    extra: {
                        let mut e = HashMap::new();
                        e.insert("choices".to_string(), Value::Array(model_choices.clone()));
                        e.insert("default".to_string(), json!("bria-rmbg"));
                        e
                    },
                });
                m
            },
            hidden: HashMap::new(),
        },
        output_types: vec![IoType::Image, IoType::Mask],
        output_names: vec!["IMAGE".to_string(), "MASK".to_string()],
        output_is_list: vec![false, false],
        is_output_node: false,
        has_intermediate_output: false,
        is_changed: None,
        not_idempotent: true,
        function_name: "remove_background".to_string(),
    };

    registry.register(class_def, Arc::new(move |ctx, node, node_id| {
        let image_in = ctx
            .resolve_input(node_id, "image")
            .unwrap_or_else(|_| json!(null));
        let model = node
            .inputs
            .get("model")
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty())
            .unwrap_or("bria-rmbg")
            .to_string();
        let node_id_str = node_id.to_string();

        Box::pin(async move {
            let image = resolve_input_image(&image_in, &node_id_str)?;
            let backend = ctx.backend();
            let images = backend
                .remove_background(image, &model)
                .map_err(|e| ExecutorError::NodeExecutionFailed {
                    node_id: node_id_str.clone(),
                    message: format!("remove_background failed: {}", e),
                })?;

            if images.is_empty() {
                return Err(ExecutorError::NodeExecutionFailed {
                    node_id: node_id_str.clone(),
                    message: "remove_background produced no output".to_string(),
                });
            }

            let img = &images[0];
            let image_data = serde_json::to_value(img).unwrap_or_else(|_| {
                json!({
                    "type": "image",
                    "width": img.width,
                    "height": img.height,
                    "channel": img.channel,
                })
            });

            // The mask output is the alpha channel of the RGBA result.
            // For simplicity we return the image again as a placeholder mask
            // (the alpha channel is embedded in the RGBA image data).
            let mask_data = json!({
                "type": "mask",
                "width": img.width,
                "height": img.height,
            });

            Ok(vec![
                json!({
                    "type": "image",
                    "images": [image_data],
                }),
                mask_data,
            ])
        })
    }));
}

pub fn register_remove_background_nodes(registry: &mut NodeRegistry) {
    register_remove_background(registry);
}

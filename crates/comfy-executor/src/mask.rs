use comfy_core::{InputTypeSpec, IoType, NodeClassDef, NodeInputTypes};
use comfy_inference::SdImage;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::Arc;

use crate::error::ExecutorError;
use crate::execution_context::ExecutionContext;
use crate::registry::NodeRegistry;

fn resolve_image(ctx: &ExecutionContext, node_id: &str, input_name: &str) -> Result<SdImage, ExecutorError> {
    let image_val = ctx.resolve_input(node_id, input_name)
        .unwrap_or_else(|_| json!(null));

    if image_val.is_null() {
        return Err(ExecutorError::NodeExecutionFailed {
            node_id: node_id.to_string(),
            message: format!("Input '{}' is required but not provided", input_name),
        });
    }

    resolve_image_from_value(&image_val).map_err(|e| ExecutorError::NodeExecutionFailed {
        node_id: node_id.to_string(),
        message: e,
    })
}

fn resolve_image_from_value(image_val: &Value) -> Result<SdImage, String> {
    if let Some(images) = image_val.get("images").and_then(|v| v.as_array()) {
        if let Some(first) = images.first() {
            return serde_json::from_value::<SdImage>(first.clone())
                .map_err(|e| format!("Failed to deserialize SdImage: {}", e));
        }
    }
    serde_json::from_value::<SdImage>(image_val.clone())
        .map_err(|e| format!("Failed to deserialize SdImage: {}", e))
}

fn resolve_mask(ctx: &ExecutionContext, node_id: &str, input_name: &str) -> Result<SdImage, ExecutorError> {
    let mask_val = ctx.resolve_input(node_id, input_name)
        .unwrap_or_else(|_| json!(null));

    if mask_val.is_null() {
        return Err(ExecutorError::NodeExecutionFailed {
            node_id: node_id.to_string(),
            message: format!("Input '{}' is required but not provided", input_name),
        });
    }

    resolve_mask_from_value(&mask_val).map_err(|e| ExecutorError::NodeExecutionFailed {
        node_id: node_id.to_string(),
        message: e,
    })
}

fn resolve_mask_from_value(mask_val: &Value) -> Result<SdImage, String> {
    if let Some(images) = mask_val.get("images").and_then(|v| v.as_array()) {
        if let Some(first) = images.first() {
            return serde_json::from_value::<SdImage>(first.clone())
                .map_err(|e| format!("Failed to deserialize SdImage: {}", e));
        }
    }
    serde_json::from_value::<SdImage>(mask_val.clone())
        .map_err(|e| format!("Failed to deserialize mask: {}", e))
}

fn mask_output(sd_image: SdImage) -> Result<Vec<Value>, ExecutorError> {
    let val = serde_json::to_value(&sd_image).map_err(|e| ExecutorError::NodeExecutionFailed {
        node_id: String::new(),
        message: format!("Failed to serialize mask: {}", e),
    })?;
    Ok(vec![json!({
        "type": "mask",
        "images": [val],
    })])
}

fn image_output(sd_image: SdImage) -> Result<Vec<Value>, ExecutorError> {
    let val = serde_json::to_value(&sd_image).map_err(|e| ExecutorError::NodeExecutionFailed {
        node_id: String::new(),
        message: format!("Failed to serialize image: {}", e),
    })?;
    Ok(vec![json!({
        "type": "image",
        "images": [val],
    })])
}

pub fn register_mask_nodes(registry: &mut NodeRegistry) {
    register_load_image_mask(registry);
    register_image_to_mask(registry);
    register_mask_to_image(registry);
    register_invert_mask(registry);
    register_mask_composite(registry);
    register_solid_mask(registry);
    register_feather_mask(registry);
    register_threshold_mask(registry);
    register_set_latent_noise_mask(registry);
}

fn register_load_image_mask(registry: &mut NodeRegistry) {
    let class_def = NodeClassDef {
        class_type: "LoadImageMask".to_string(),
        display_name: "Load Image (as Mask)".to_string(),
        category: "mask".to_string(),
        input_types: NodeInputTypes {
            required: {
                let mut m = HashMap::new();
                m.insert("image".to_string(), InputTypeSpec {
                    type_name: "COMBO".to_string(),
                    extra: HashMap::new(),
                });
                m.insert("channel".to_string(), InputTypeSpec {
                    type_name: "COMBO".to_string(),
                    extra: HashMap::new(),
                });
                m
            },
            optional: HashMap::new(),
            hidden: HashMap::new(),
        },
        output_types: vec![IoType::Mask],
        output_names: vec!["MASK".to_string()],
        output_is_list: vec![false],
        is_output_node: false,
        has_intermediate_output: false,
        is_changed: None,
        not_idempotent: false,
        function_name: "load_mask".to_string(),
    };

    registry.register(class_def, Arc::new(|_ctx, node, _node_id| {
        let image_path = node.inputs.get("image")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        Box::pin(async move {
            Ok(vec![json!({
                "type": "mask",
                "path": image_path,
            })])
        })
    }));
}

fn register_image_to_mask(registry: &mut NodeRegistry) {
    let class_def = NodeClassDef {
        class_type: "ImageToMask".to_string(),
        display_name: "Image to Mask".to_string(),
        category: "mask".to_string(),
        input_types: NodeInputTypes {
            required: {
                let mut m = HashMap::new();
                m.insert("image".to_string(), InputTypeSpec {
                    type_name: "IMAGE".to_string(),
                    extra: HashMap::new(),
                });
                m.insert("channel".to_string(), InputTypeSpec {
                    type_name: "COMBO".to_string(),
                    extra: HashMap::new(),
                });
                m
            },
            optional: HashMap::new(),
            hidden: HashMap::new(),
        },
        output_types: vec![IoType::Mask],
        output_names: vec!["MASK".to_string()],
        output_is_list: vec![false],
        is_output_node: false,
        has_intermediate_output: false,
        is_changed: None,
        not_idempotent: false,
        function_name: "convert".to_string(),
    };

    registry.register(class_def, Arc::new(|ctx, _node, node_id| {
        let sd_image = match resolve_image(ctx, node_id, "image") {
            Ok(img) => img,
            Err(e) => return Box::pin(async move { Err(e) }),
        };
        let channel = ctx.resolve_input(node_id, "channel")
            .ok()
            .and_then(|v| v.as_str().map(|s| s.to_string()))
            .unwrap_or_else(|| "red".to_string());

        Box::pin(async move {
            let mask = image_to_channel_mask(&sd_image, &channel);
            mask_output(mask)
        })
    }));
}

fn register_mask_to_image(registry: &mut NodeRegistry) {
    let class_def = NodeClassDef {
        class_type: "MaskToImage".to_string(),
        display_name: "Mask to Image".to_string(),
        category: "mask".to_string(),
        input_types: NodeInputTypes {
            required: {
                let mut m = HashMap::new();
                m.insert("mask".to_string(), InputTypeSpec {
                    type_name: "MASK".to_string(),
                    extra: HashMap::new(),
                });
                m
            },
            optional: HashMap::new(),
            hidden: HashMap::new(),
        },
        output_types: vec![IoType::Image],
        output_names: vec!["IMAGE".to_string()],
        output_is_list: vec![false],
        is_output_node: false,
        has_intermediate_output: false,
        is_changed: None,
        not_idempotent: false,
        function_name: "convert".to_string(),
    };

    registry.register(class_def, Arc::new(|ctx, _node, node_id| {
        let mask = match resolve_mask(ctx, node_id, "mask") {
            Ok(m) => m,
            Err(e) => return Box::pin(async move { Err(e) }),
        };

        Box::pin(async move {
            let img = mask_to_rgb_image(&mask);
            image_output(img)
        })
    }));
}

fn register_invert_mask(registry: &mut NodeRegistry) {
    let class_def = NodeClassDef {
        class_type: "InvertMask".to_string(),
        display_name: "Invert Mask".to_string(),
        category: "mask".to_string(),
        input_types: NodeInputTypes {
            required: {
                let mut m = HashMap::new();
                m.insert("mask".to_string(), InputTypeSpec {
                    type_name: "MASK".to_string(),
                    extra: HashMap::new(),
                });
                m
            },
            optional: HashMap::new(),
            hidden: HashMap::new(),
        },
        output_types: vec![IoType::Mask],
        output_names: vec!["MASK".to_string()],
        output_is_list: vec![false],
        is_output_node: false,
        has_intermediate_output: false,
        is_changed: None,
        not_idempotent: false,
        function_name: "invert".to_string(),
    };

    registry.register(class_def, Arc::new(|ctx, _node, node_id| {
        let mask = match resolve_mask(ctx, node_id, "mask") {
            Ok(m) => m,
            Err(e) => return Box::pin(async move { Err(e) }),
        };

        Box::pin(async move {
            let inverted = invert_mask(&mask);
            mask_output(inverted)
        })
    }));
}

fn register_mask_composite(registry: &mut NodeRegistry) {
    let class_def = NodeClassDef {
        class_type: "MaskComposite".to_string(),
        display_name: "Mask Composite".to_string(),
        category: "mask".to_string(),
        input_types: NodeInputTypes {
            required: {
                let mut m = HashMap::new();
                m.insert("destination".to_string(), InputTypeSpec {
                    type_name: "MASK".to_string(),
                    extra: HashMap::new(),
                });
                m.insert("source".to_string(), InputTypeSpec {
                    type_name: "MASK".to_string(),
                    extra: HashMap::new(),
                });
                m.insert("operation".to_string(), InputTypeSpec {
                    type_name: "COMBO".to_string(),
                    extra: HashMap::new(),
                });
                m.insert("x".to_string(), InputTypeSpec {
                    type_name: "INT".to_string(),
                    extra: HashMap::new(),
                });
                m.insert("y".to_string(), InputTypeSpec {
                    type_name: "INT".to_string(),
                    extra: HashMap::new(),
                });
                m
            },
            optional: HashMap::new(),
            hidden: HashMap::new(),
        },
        output_types: vec![IoType::Mask],
        output_names: vec!["MASK".to_string()],
        output_is_list: vec![false],
        is_output_node: false,
        has_intermediate_output: false,
        is_changed: None,
        not_idempotent: false,
        function_name: "composite".to_string(),
    };

    registry.register(class_def, Arc::new(|ctx, _node, node_id| {
        let dest = match resolve_mask(ctx, node_id, "destination") {
            Ok(m) => m,
            Err(e) => return Box::pin(async move { Err(e) }),
        };
        let src = match resolve_mask(ctx, node_id, "source") {
            Ok(m) => m,
            Err(e) => return Box::pin(async move { Err(e) }),
        };
        let operation = ctx.resolve_input(node_id, "operation")
            .unwrap_or_else(|_| json!("add"))
            .as_str()
            .unwrap_or("add")
            .to_string();
        let x = ctx.resolve_input(node_id, "x")
            .unwrap_or_else(|_| json!(0))
            .as_i64()
            .unwrap_or(0) as i32;
        let y = ctx.resolve_input(node_id, "y")
            .unwrap_or_else(|_| json!(0))
            .as_i64()
            .unwrap_or(0) as i32;

        Box::pin(async move {
            let result = composite_masks(&dest, &src, &operation, x, y);
            mask_output(result)
        })
    }));
}

fn register_solid_mask(registry: &mut NodeRegistry) {
    let class_def = NodeClassDef {
        class_type: "SolidMask".to_string(),
        display_name: "Solid Mask".to_string(),
        category: "mask".to_string(),
        input_types: NodeInputTypes {
            required: {
                let mut m = HashMap::new();
                m.insert("value".to_string(), InputTypeSpec {
                    type_name: "FLOAT".to_string(),
                    extra: HashMap::new(),
                });
                m.insert("width".to_string(), InputTypeSpec {
                    type_name: "INT".to_string(),
                    extra: HashMap::new(),
                });
                m.insert("height".to_string(), InputTypeSpec {
                    type_name: "INT".to_string(),
                    extra: HashMap::new(),
                });
                m
            },
            optional: HashMap::new(),
            hidden: HashMap::new(),
        },
        output_types: vec![IoType::Mask],
        output_names: vec!["MASK".to_string()],
        output_is_list: vec![false],
        is_output_node: false,
        has_intermediate_output: false,
        is_changed: None,
        not_idempotent: false,
        function_name: "generate".to_string(),
    };

    registry.register(class_def, Arc::new(|_ctx, node, _node_id| {
        let value = node.inputs.get("value")
            .and_then(|v| v.as_f64())
            .unwrap_or(1.0) as f32;
        let width = node.inputs.get("width")
            .and_then(|v| v.as_i64())
            .unwrap_or(512) as u32;
        let height = node.inputs.get("height")
            .and_then(|v| v.as_i64())
            .unwrap_or(512) as u32;

        Box::pin(async move {
            let mask = create_solid_mask(width, height, value);
            mask_output(mask)
        })
    }));
}

fn register_feather_mask(registry: &mut NodeRegistry) {
    let class_def = NodeClassDef {
        class_type: "FeatherMask".to_string(),
        display_name: "Feather Mask".to_string(),
        category: "mask".to_string(),
        input_types: NodeInputTypes {
            required: {
                let mut m = HashMap::new();
                m.insert("mask".to_string(), InputTypeSpec {
                    type_name: "MASK".to_string(),
                    extra: HashMap::new(),
                });
                m.insert("left".to_string(), InputTypeSpec {
                    type_name: "INT".to_string(),
                    extra: HashMap::new(),
                });
                m.insert("top".to_string(), InputTypeSpec {
                    type_name: "INT".to_string(),
                    extra: HashMap::new(),
                });
                m.insert("right".to_string(), InputTypeSpec {
                    type_name: "INT".to_string(),
                    extra: HashMap::new(),
                });
                m.insert("bottom".to_string(), InputTypeSpec {
                    type_name: "INT".to_string(),
                    extra: HashMap::new(),
                });
                m
            },
            optional: HashMap::new(),
            hidden: HashMap::new(),
        },
        output_types: vec![IoType::Mask],
        output_names: vec!["MASK".to_string()],
        output_is_list: vec![false],
        is_output_node: false,
        has_intermediate_output: false,
        is_changed: None,
        not_idempotent: false,
        function_name: "feather".to_string(),
    };

    registry.register(class_def, Arc::new(|ctx, _node, node_id| {
        let mask = match resolve_mask(ctx, node_id, "mask") {
            Ok(m) => m,
            Err(e) => return Box::pin(async move { Err(e) }),
        };
        let left = ctx.resolve_input(node_id, "left")
            .unwrap_or_else(|_| json!(0))
            .as_i64()
            .unwrap_or(0) as u32;
        let top = ctx.resolve_input(node_id, "top")
            .unwrap_or_else(|_| json!(0))
            .as_i64()
            .unwrap_or(0) as u32;
        let right = ctx.resolve_input(node_id, "right")
            .unwrap_or_else(|_| json!(0))
            .as_i64()
            .unwrap_or(0) as u32;
        let bottom = ctx.resolve_input(node_id, "bottom")
            .unwrap_or_else(|_| json!(0))
            .as_i64()
            .unwrap_or(0) as u32;

        Box::pin(async move {
            let result = feather_mask(&mask, left, top, right, bottom);
            mask_output(result)
        })
    }));
}

fn register_threshold_mask(registry: &mut NodeRegistry) {
    let class_def = NodeClassDef {
        class_type: "ThresholdMask".to_string(),
        display_name: "Threshold Mask".to_string(),
        category: "mask".to_string(),
        input_types: NodeInputTypes {
            required: {
                let mut m = HashMap::new();
                m.insert("mask".to_string(), InputTypeSpec {
                    type_name: "MASK".to_string(),
                    extra: HashMap::new(),
                });
                m.insert("value".to_string(), InputTypeSpec {
                    type_name: "FLOAT".to_string(),
                    extra: HashMap::new(),
                });
                m
            },
            optional: HashMap::new(),
            hidden: HashMap::new(),
        },
        output_types: vec![IoType::Mask],
        output_names: vec!["MASK".to_string()],
        output_is_list: vec![false],
        is_output_node: false,
        has_intermediate_output: false,
        is_changed: None,
        not_idempotent: false,
        function_name: "threshold".to_string(),
    };

    registry.register(class_def, Arc::new(|ctx, _node, node_id| {
        let mask = match resolve_mask(ctx, node_id, "mask") {
            Ok(m) => m,
            Err(e) => return Box::pin(async move { Err(e) }),
        };
        let value = ctx.resolve_input(node_id, "value")
            .unwrap_or_else(|_| json!(0.5))
            .as_f64()
            .unwrap_or(0.5) as f32;

        Box::pin(async move {
            let result = threshold_mask(&mask, value);
            mask_output(result)
        })
    }));
}

fn register_set_latent_noise_mask(registry: &mut NodeRegistry) {
    let class_def = NodeClassDef {
        class_type: "SetLatentNoiseMask".to_string(),
        display_name: "Set Latent Noise Mask".to_string(),
        category: "latent".to_string(),
        input_types: NodeInputTypes {
            required: {
                let mut m = HashMap::new();
                m.insert("samples".to_string(), InputTypeSpec {
                    type_name: "LATENT".to_string(),
                    extra: HashMap::new(),
                });
                m.insert("mask".to_string(), InputTypeSpec {
                    type_name: "MASK".to_string(),
                    extra: HashMap::new(),
                });
                m
            },
            optional: HashMap::new(),
            hidden: HashMap::new(),
        },
        output_types: vec![IoType::Latent],
        output_names: vec!["LATENT".to_string()],
        output_is_list: vec![false],
        is_output_node: false,
        has_intermediate_output: false,
        is_changed: None,
        not_idempotent: false,
        function_name: "set_mask".to_string(),
    };

    registry.register(class_def, Arc::new(|ctx, _node, node_id| {
        let samples = ctx.resolve_input(node_id, "samples")
            .unwrap_or_else(|_| json!({}));
        let mask = ctx.resolve_input(node_id, "mask")
            .unwrap_or_else(|_| json!(null));

        Box::pin(async move {
            let mut result = samples.as_object().cloned().unwrap_or_default();
            result.insert("noise_mask".to_string(), mask);
            Ok(vec![json!(result)])
        })
    }));
}

fn image_to_channel_mask(image: &SdImage, channel: &str) -> SdImage {
    let channel_idx = match channel {
        "red" => 0,
        "green" => 1,
        "blue" => 2,
        "alpha" => 3,
        _ => 0,
    };

    let gray_data: Vec<u8> = match image.channel {
        1 => image.data.clone(),
        3 => {
            if channel_idx < 3 {
                image.data.chunks(3).map(|px| px[channel_idx]).collect()
            } else {
                image.data.chunks(3).map(|px| {
                    ((px[0] as u32 * 299 + px[1] as u32 * 587 + px[2] as u32 * 114) / 1000) as u8
                }).collect()
            }
        }
        4 => {
            if channel_idx < 4 {
                image.data.chunks(4).map(|px| px[channel_idx]).collect()
            } else {
                image.data.chunks(4).map(|px| {
                    ((px[0] as u32 * 299 + px[1] as u32 * 587 + px[2] as u32 * 114) / 1000) as u8
                }).collect()
            }
        }
        _ => image.data.clone(),
    };
    SdImage::grayscale(image.width, image.height, gray_data)
        .unwrap_or_else(|_| SdImage::new(image.width, image.height, 1))
}

fn image_to_grayscale_mask(image: &SdImage) -> SdImage {
    let gray_data: Vec<u8> = match image.channel {
        1 => image.data.clone(),
        3 => image.data.chunks(3).map(|px| {
            ((px[0] as u32 * 299 + px[1] as u32 * 587 + px[2] as u32 * 114) / 1000) as u8
        }).collect(),
        4 => image.data.chunks(4).map(|px| {
            ((px[0] as u32 * 299 + px[1] as u32 * 587 + px[2] as u32 * 114) / 1000) as u8
        }).collect(),
        _ => image.data.clone(),
    };
    SdImage::grayscale(image.width, image.height, gray_data)
        .unwrap_or_else(|_| SdImage::new(image.width, image.height, 1))
}

fn mask_to_rgb_image(mask: &SdImage) -> SdImage {
    let gray = image_to_grayscale_mask(mask);
    let rgb_data: Vec<u8> = gray.data.iter().flat_map(|&v| [v, v, v]).collect();
    SdImage::rgb(mask.width, mask.height, rgb_data)
        .unwrap_or_else(|_| SdImage::new(mask.width, mask.height, 3))
}

fn invert_mask(mask: &SdImage) -> SdImage {
    let gray = image_to_grayscale_mask(mask);
    let inverted: Vec<u8> = gray.data.iter().map(|&v| 255 - v).collect();
    SdImage::grayscale(mask.width, mask.height, inverted)
        .unwrap_or_else(|_| SdImage::new(mask.width, mask.height, 1))
}

fn composite_masks(dest: &SdImage, src: &SdImage, operation: &str, x: i32, y: i32) -> SdImage {
    let dest_gray = image_to_grayscale_mask(dest);
    let src_gray = image_to_grayscale_mask(src);

    let width = dest_gray.width;
    let height = dest_gray.height;
    let mut result = dest_gray.data.clone();

    for sy in 0..src_gray.height {
        let dy = y + sy as i32;
        if dy < 0 || dy >= height as i32 {
            continue;
        }
        for sx in 0..src_gray.width {
            let dx = x + sx as i32;
            if dx < 0 || dx >= width as i32 {
                continue;
            }
            let dest_val = result[(dy as u32 * width + dx as u32) as usize] as f32 / 255.0;
            let src_val = src_gray.data[(sy * src_gray.width + sx) as usize] as f32 / 255.0;
            let new_val = match operation {
                "multiply" => dest_val * src_val,
                "add" => (dest_val + src_val).min(1.0),
                "subtract" => (dest_val - src_val).max(0.0),
                "intersect" => dest_val.min(src_val),
                "difference" => (dest_val - src_val).abs(),
                "divide" => {
                    if src_val > 0.0 { (dest_val / src_val).min(1.0) } else { 1.0 }
                }
                _ => (dest_val + src_val).min(1.0),
            };
            result[(dy as u32 * width + dx as u32) as usize] = (new_val * 255.0) as u8;
        }
    }

    SdImage::grayscale(width, height, result)
        .unwrap_or_else(|_| SdImage::new(width, height, 1))
}

fn create_solid_mask(width: u32, height: u32, value: f32) -> SdImage {
    let v = (value.max(0.0).min(1.0) * 255.0) as u8;
    let data = vec![v; (width * height) as usize];
    SdImage::grayscale(width, height, data)
        .unwrap_or_else(|_| SdImage::new(width, height, 1))
}

fn feather_mask(mask: &SdImage, left: u32, top: u32, right: u32, bottom: u32) -> SdImage {
    let gray = image_to_grayscale_mask(mask);
    let width = gray.width;
    let height = gray.height;
    let mut result = gray.data.clone();

    for y in 0..height {
        for x in 0..width {
            let mut alpha = 1.0f32;
            if left > 0 && x < left {
                alpha = alpha * (x as f32 / left as f32);
            }
            if top > 0 && y < top {
                alpha = alpha * (y as f32 / top as f32);
            }
            if right > 0 && x >= width - right {
                alpha = alpha * ((width - 1 - x) as f32 / right as f32);
            }
            if bottom > 0 && y >= height - bottom {
                alpha = alpha * ((height - 1 - y) as f32 / bottom as f32);
            }
            let idx = (y * width + x) as usize;
            result[idx] = (result[idx] as f32 * alpha).min(255.0) as u8;
        }
    }

    SdImage::grayscale(width, height, result)
        .unwrap_or_else(|_| SdImage::new(width, height, 1))
}

fn threshold_mask(mask: &SdImage, value: f32) -> SdImage {
    let gray = image_to_grayscale_mask(mask);
    let thresh = (value.max(0.0).min(1.0) * 255.0) as u8;
    let result: Vec<u8> = gray.data.iter().map(|&v| {
        if v >= thresh { 255 } else { 0 }
    }).collect();
    SdImage::grayscale(mask.width, mask.height, result)
        .unwrap_or_else(|_| SdImage::new(mask.width, mask.height, 1))
}

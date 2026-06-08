use crate::error::ExecutorError;
use crate::registry::NodeRegistry;
use comfy_core::{IoType, NodeClassDef, NodeInputTypes, InputTypeSpec};
use serde_json::json;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;

/// Scan input images from the input_images directory
fn scan_input_images() -> Vec<String> {
    let input_dir = std::env::var("COMFY_INPUT_DIR")
        .unwrap_or_else(|_| "input".to_string());
    let path = PathBuf::from(&input_dir);
    
    if !path.exists() {
        return vec![];
    }

    fs::read_dir(path)
        .ok()
        .into_iter()
        .flatten()
        .filter_map(|e| e.ok())
        .filter_map(|e| {
            let name = e.file_name().to_string_lossy().to_string();
            let ext = e.path().extension()?.to_string_lossy().to_lowercase();
            ["png", "jpg", "jpeg", "bmp", "tiff", "webp"].contains(&ext.as_str())
                .then(|| name)
        })
        .collect()
}

/// Get the output directory for 3D results
fn get_output_3d_dir() -> PathBuf {
    let base = std::env::var("COMFY_OUTPUT_DIR")
        .unwrap_or_else(|_| "output".to_string());
    let base_path = PathBuf::from(&base);
    let output_3d = base_path.join("3d_gaussians");
    
    if !output_3d.exists() {
        let _ = fs::create_dir_all(&output_3d);
    }
    
    output_3d
}

/// Get the models directory for TripoSplat checkpoints
fn get_triposplat_models_dir() -> PathBuf {
    let base = std::env::var("COMFY_MODELS_DIR")
        .unwrap_or_else(|_| "models".to_string());
    let base_path = PathBuf::from(&base);
    base_path.join("triposplat")
}

fn scan_triposplat_models() -> Vec<String> {
    let model_dir = get_triposplat_models_dir();
    if !model_dir.exists() {
        return vec![];
    }

    fs::read_dir(model_dir)
        .ok()
        .into_iter()
        .flatten()
        .filter_map(|e| e.ok())
        .filter_map(|e| {
            let name = e.file_name().to_string_lossy().to_string();
            let ext = e.path().extension()?.to_string_lossy().to_lowercase();
            ["safetensors", "gguf"].contains(&ext.as_str())
                .then(|| name)
        })
        .collect()
}

fn register_triposplat_pipeline(registry: &mut NodeRegistry) {
    let _image_choices = scan_input_images(); // Reserved for future image file selection
    let model_choices = scan_triposplat_models();

    let class_def = NodeClassDef {
        class_type: "TripoSplatPipeline".to_string(),
        display_name: "TripoSplat 2D to 3D".to_string(),
        category: "3d/triposplat".to_string(),
        input_types: NodeInputTypes {
            required: {
                let mut m = HashMap::new();
                m.insert("image".to_string(), InputTypeSpec {
                    type_name: "IMAGE".to_string(),
                    extra: HashMap::new(),
                });
                m.insert("triposplat_model_name".to_string(), InputTypeSpec {
                    type_name: "COMBO".to_string(),
                    extra: {
                        let mut e = HashMap::new();
                        e.insert("choices".to_string(), serde_json::Value::Array(
                            model_choices.iter().map(|s| json!(s)).collect()
                        ));
                        e
                    },
                });
                m
            },
            optional: {
                let mut m = HashMap::new();
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
                        e.insert("default".to_string(), json!(20));
                        e
                    },
                });
                m.insert("guidance_scale".to_string(), InputTypeSpec {
                    type_name: "FLOAT".to_string(),
                    extra: {
                        let mut e = HashMap::new();
                        e.insert("min".to_string(), json!(1.0));
                        e.insert("max".to_string(), json!(20.0));
                        e.insert("step".to_string(), json!(0.1));
                        e.insert("default".to_string(), json!(3.0));
                        e
                    },
                });
                m.insert("num_gaussians".to_string(), InputTypeSpec {
                    type_name: "INT".to_string(),
                    extra: {
                        let mut e = HashMap::new();
                        e.insert("min".to_string(), json!(32768));
                        e.insert("max".to_string(), json!(262144));
                        e.insert("step".to_string(), json!(32768));
                        e.insert("default".to_string(), json!(262144));
                        e
                    },
                });
                m.insert("output_format".to_string(), InputTypeSpec {
                    type_name: "COMBO".to_string(),
                    extra: {
                        let mut e = HashMap::new();
                        e.insert("choices".to_string(), json!(["ply", "splat"]));
                        e
                    },
                });
                m
            },
            hidden: HashMap::new(),
        },
        output_types: vec![IoType::Gaussian3D, IoType::Image],
        output_names: vec!["GAUSSIANS".to_string(), "PREPARED_IMAGE".to_string()],
        output_is_list: vec![false, false],
        is_output_node: false,
        has_intermediate_output: false,
        is_changed: None,
        not_idempotent: false,
        function_name: "run".to_string(),
    };

    registry.register(class_def, Arc::new(|ctx, node, node_id| {
        let image = ctx.resolve_input(node_id, "image")
            .unwrap_or_else(|_| json!({}));
        let model_name = node.inputs.get("triposplat_model_name")
            .and_then(|v| v.as_str())
            .unwrap_or("triposplat.safetensors");
        
        let seed = node.inputs.get("seed")
            .and_then(|v| v.as_i64())
            .unwrap_or(42) as u32;
        let steps = node.inputs.get("steps")
            .and_then(|v| v.as_i64())
            .unwrap_or(20) as u32;
        let guidance_scale = node.inputs.get("guidance_scale")
            .and_then(|v| v.as_f64())
            .unwrap_or(3.0);
        let num_gaussians = node.inputs.get("num_gaussians")
            .and_then(|v| v.as_i64())
            .unwrap_or(262144) as u32;
        let output_format = node.inputs.get("output_format")
            .and_then(|v| v.as_str())
            .unwrap_or("ply");
        let node_id_str = node_id.to_string();

        Box::pin(async move {
            // Extract image path
            let image_path = image.get("path")
                .and_then(|v| v.as_str())
                .unwrap_or("");

            if image_path.is_empty() {
                return Err(ExecutorError::NodeExecutionFailed {
                    node_id: node_id_str.clone(),
                    message: "No valid image input provided".to_string(),
                });
            }

            // Get model path
            let models_dir = get_triposplat_models_dir();
            let model_path = models_dir.join(model_name);
            
            if !model_path.exists() {
                return Err(ExecutorError::NodeExecutionFailed {
                    node_id: node_id_str.clone(),
                    message: format!("TripoSplat model not found: {}", model_path.display()),
                });
            }

            // Call TripoSplat via Python subprocess
            let output_dir = get_output_3d_dir();
            let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S");
            let output_filename = format!("triposplat_{}_{}.{}", timestamp, seed, output_format);
            let output_path = output_dir.join(&output_filename);

            tracing::info!("TripoSplat: processing image={} model={} output={}", 
                          image_path, model_path.display(), output_path.display());

            // Build Python command to call TripoSplat
            let python_cmd = std::env::var("TRIPOPLAT_PYTHON").unwrap_or_else(|_| "python".to_string());
            
            // Find the TripoSplat reference script
            let script_path = std::env::var("TRIPOPLAT_SCRIPT").unwrap_or_else(|_| {
                // Default path relative to project
                "reference/TripoSplat/run_example.py".to_string()
            });

            let output = std::process::Command::new(&python_cmd)
                .args([
                    &script_path,
                    "--input", image_path,
                    "--output", &output_path.to_string_lossy(),
                    "--model", &model_path.to_string_lossy(),
                    "--steps", &steps.to_string(),
                    "--guidance_scale", &guidance_scale.to_string(),
                    "--num_gaussians", &num_gaussians.to_string(),
                    "--seed", &seed.to_string(),
                ])
                .output();

            match output {
                Ok(output) => {
                    if output.status.success() {
                        tracing::info!("TripoSplat: successfully generated {}", output_path.display());
                        
                        Ok(vec![
                            json!({
                                "type": "gaussian_3d",
                                "file_path": output_path.to_string_lossy().to_string(),
                                "format": output_format,
                                "num_gaussians": num_gaussians,
                                "seed": seed,
                            }),
                            json!({
                                "type": "image",
                                "path": image_path,
                            }),
                        ])
                    } else {
                        let stderr = String::from_utf8_lossy(&output.stderr);
                        tracing::error!("TripoSplat failed: {}", stderr);
                        Err(ExecutorError::NodeExecutionFailed {
                            node_id: node_id_str.clone(),
                            message: format!("TripoSplat execution failed: {}", stderr),
                        })
                    }
                }
                Err(e) => {
                    tracing::error!("Failed to execute TripoSplat: {}", e);
                    Err(ExecutorError::NodeExecutionFailed {
                        node_id: node_id_str.clone(),
                        message: format!("Failed to execute TripoSplat: {}", e),
                    })
                }
            }
        })
    }));
}

fn register_ply_output(registry: &mut NodeRegistry) {
    let class_def = NodeClassDef {
        class_type: "Gaussian3DPreview".to_string(),
        display_name: "3D Gaussian Preview".to_string(),
        category: "3d/triposplat".to_string(),
        input_types: NodeInputTypes {
            required: {
                let mut m = HashMap::new();
                m.insert("gaussians".to_string(), InputTypeSpec {
                    type_name: "GAUSSIAN_3D".to_string(),
                    extra: HashMap::new(),
                });
                m
            },
            optional: HashMap::new(),
            hidden: HashMap::new(),
        },
        output_types: vec![IoType::Image],
        output_names: vec!["PREVIEW".to_string()],
        output_is_list: vec![false],
        is_output_node: true,
        has_intermediate_output: false,
        is_changed: None,
        not_idempotent: false,
        function_name: "preview".to_string(),
    };

    registry.register(class_def, Arc::new(|ctx, _node, node_id| {
        let gaussians = ctx.resolve_input(node_id, "gaussians")
            .unwrap_or_else(|_| json!({}));

        Box::pin(async move {
            let file_path = gaussians.get("file_path")
                .and_then(|v| v.as_str())
                .unwrap_or("");

            Ok(vec![json!({
                "type": "image",
                "path": file_path,
                "preview_type": "3d_gaussian",
            })])
        })
    }));
}

pub fn register_triposplat_nodes(registry: &mut NodeRegistry) {
    register_triposplat_pipeline(registry);
    register_ply_output(registry);
}

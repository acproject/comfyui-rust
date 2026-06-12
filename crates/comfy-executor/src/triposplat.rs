use crate::error::ExecutorError;
use crate::registry::NodeRegistry;
use comfy_core::{IoType, NodeClassDef, NodeInputTypes, InputTypeSpec};
use comfy_inference::image::SdImage;
use comfy_inference::params::{Gaussian3DParams, ModelConfig};
use serde_json::json;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;

/// Get the models base directory
fn get_models_dir() -> String {
    std::env::var("COMFY_MODELS_DIR").unwrap_or_else(|_| "models".to_string())
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

/// Scan model files in a subdirectory, filtering by keyword
fn scan_models(sub_dir: &str, keyword: Option<&str>) -> Vec<String> {
    let models_dir = get_models_dir();
    let dir_path = PathBuf::from(&models_dir).join(sub_dir);
    if !dir_path.exists() {
        return vec![];
    }

    fs::read_dir(&dir_path)
        .ok()
        .into_iter()
        .flatten()
        .filter_map(|e| e.ok())
        .filter_map(|e| {
            let name = e.file_name().to_string_lossy().to_string();
            let ext = e.path().extension()?.to_string_lossy().to_lowercase();
            if ["safetensors", "gguf", "pt", "bin"].contains(&ext.as_str()) {
                if let Some(kw) = keyword {
                    if name.to_lowercase().contains(kw) {
                        Some(name)
                    } else {
                        None
                    }
                } else {
                    Some(name)
                }
            } else {
                None
            }
        })
        .collect()
}

fn register_triposplat_pipeline(registry: &mut NodeRegistry) {
    let model_choices = scan_models("diffusion_models", Some("triposplat"));
    let rmbg_choices = scan_models("background_removal", Some("birefnet"));
    let clip_vision_choices = scan_models("clip_vision", Some("dino_v3"));
    let vae_choices = scan_models("vae", Some("flux2-vae"));
    let decoder_choices = scan_models("background_removal", Some("triposplat_vae_decoder"));

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
                m.insert("rmbg_model_name".to_string(), InputTypeSpec {
                    type_name: "COMBO".to_string(),
                    extra: {
                        let mut e = HashMap::new();
                        e.insert("choices".to_string(), serde_json::Value::Array(
                            rmbg_choices.iter().map(|s| json!(s)).collect()
                        ));
                        e
                    },
                });
                m.insert("clip_vision_model_name".to_string(), InputTypeSpec {
                    type_name: "COMBO".to_string(),
                    extra: {
                        let mut e = HashMap::new();
                        e.insert("choices".to_string(), serde_json::Value::Array(
                            clip_vision_choices.iter().map(|s| json!(s)).collect()
                        ));
                        e
                    },
                });
                m.insert("vae_model_name".to_string(), InputTypeSpec {
                    type_name: "COMBO".to_string(),
                    extra: {
                        let mut e = HashMap::new();
                        e.insert("choices".to_string(), serde_json::Value::Array(
                            vae_choices.iter().map(|s| json!(s)).collect()
                        ));
                        e
                    },
                });
                m.insert("decoder_model_name".to_string(), InputTypeSpec {
                    type_name: "COMBO".to_string(),
                    extra: {
                        let mut e = HashMap::new();
                        e.insert("choices".to_string(), serde_json::Value::Array(
                            decoder_choices.iter().map(|s| json!(s)).collect()
                        ));
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
                m.insert("erode_radius".to_string(), InputTypeSpec {
                    type_name: "INT".to_string(),
                    extra: {
                        let mut e = HashMap::new();
                        e.insert("min".to_string(), json!(0));
                        e.insert("max".to_string(), json!(10));
                        e.insert("step".to_string(), json!(1));
                        e.insert("default".to_string(), json!(1));
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
            .unwrap_or("triposplat_fp16.safetensors");
        let rmbg_model = node.inputs.get("rmbg_model_name")
            .and_then(|v| v.as_str());
        let clip_vision_model = node.inputs.get("clip_vision_model_name")
            .and_then(|v| v.as_str())
            .unwrap_or("dino_v3_vit_h.safetensors");
        let vae_model = node.inputs.get("vae_model_name")
            .and_then(|v| v.as_str())
            .unwrap_or("flux2-vae.safetensors");
        let decoder_model = node.inputs.get("decoder_model_name")
            .and_then(|v| v.as_str());

        let seed = node.inputs.get("seed")
            .and_then(|v| v.as_i64())
            .unwrap_or(42) as i64;
        let steps = node.inputs.get("steps")
            .and_then(|v| v.as_i64())
            .unwrap_or(20) as i32;
        let guidance_scale = node.inputs.get("guidance_scale")
            .and_then(|v| v.as_f64())
            .unwrap_or(3.0) as f32;
        let num_gaussians = node.inputs.get("num_gaussians")
            .and_then(|v| v.as_i64())
            .unwrap_or(262144) as i32;
        let output_format = node.inputs.get("output_format")
            .and_then(|v| v.as_str())
            .unwrap_or("ply");
        let erode_radius = node.inputs.get("erode_radius")
            .and_then(|v| v.as_i64())
            .unwrap_or(1) as i32;
        let node_id_str = node_id.to_string();

        Box::pin(async move {
            // Extract image - supports both {"type":"image","path":"..."} from LoadImage
            // and {"width":..,"height":..,"data":..} from direct image data
            let sd_image = {
                // Try to get image from path first (standard LoadImage output format)
                if let Some(path) = image.get("path").and_then(|v| v.as_str()) {
                    if !path.is_empty() {
                        // Resolve path: if relative, try "input/{path}" first, then as-is
                        let resolved_path = if std::path::Path::new(path).is_absolute() {
                            path.to_string()
                        } else if path.starts_with("input/") || std::path::Path::new(path).exists() {
                            // Path already has input/ prefix or exists as-is
                            path.to_string()
                        } else {
                            let input_path = format!("input/{}", path);
                            if std::path::Path::new(&input_path).exists() {
                                input_path
                            } else {
                                path.to_string()
                            }
                        };
                        let img_bytes = fs::read(&resolved_path).map_err(|e| ExecutorError::NodeExecutionFailed {
                            node_id: node_id_str.clone(),
                            message: format!("Failed to read image file '{}': {}", resolved_path, e),
                        })?;
                        SdImage::from_png_bytes(&img_bytes).map_err(|e| ExecutorError::NodeExecutionFailed {
                            node_id: node_id_str.clone(),
                            message: format!("Failed to decode image '{}': {}", resolved_path, e),
                        })?
                    } else {
                        return Err(ExecutorError::NodeExecutionFailed {
                            node_id: node_id_str.clone(),
                            message: "No valid image input provided (empty path)".to_string(),
                        });
                    }
                } else {
                    return Err(ExecutorError::NodeExecutionFailed {
                        node_id: node_id_str.clone(),
                        message: "No valid image input provided".to_string(),
                    });
                }
            };

            // Build model config
            let models_dir = get_models_dir();
            let mut model_config = ModelConfig::new();

            // Set the main TripoSplat model path (loaded without prefix in C++,
            // needed for version detection which looks for noise_refiner/cam_refiner)
            model_config = model_config.with_model(format!("{}/diffusion_models/{}", models_dir, model_name));

            // Set clip_vision (DINOv3) path
            model_config = model_config.with_clip_vision(format!("{}/clip_vision/{}", models_dir, clip_vision_model));

            // Set VAE encoder path (Flux2 VAE)
            model_config = model_config.with_vae(format!("{}/vae/{}", models_dir, vae_model));

            if let Some(decoder_model) = decoder_model {
                model_config = model_config.with_decoder(
                    format!("{}/background_removal/{}", models_dir, decoder_model)
                );
            }

            if let Some(rmbg_model) = rmbg_model {
                model_config = model_config.with_rmbg(
                    format!("{}/background_removal/{}", models_dir, rmbg_model)
                );
            }

            // Set diffusion model path (loaded with "model.diffusion_model." prefix in C++,
            // needed for TripoSplatFlowModel which expects model.diffusion_model.* tensors)
            model_config = model_config.with_diffusion_model(
                format!("{}/diffusion_models/{}", models_dir, model_name)
            );

            // Build output path
            let output_dir = get_output_3d_dir();
            let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S");
            let output_filename = format!("triposplat_{}_{}.{}", timestamp, seed, output_format);
            let output_path = output_dir.join(&output_filename);

            // Build 3D generation params
            let params = Gaussian3DParams::new(model_config)
                .with_input_image(sd_image)
                .with_seed(seed)
                .with_steps(steps)
                .with_guidance_scale(guidance_scale)
                .with_num_gaussians(num_gaussians)
                .with_output_path(output_path.to_string_lossy().to_string())
                .with_output_format(if output_format == "splat" { 1 } else { 0 });

            // Call the inference backend
            let backend = ctx.backend();
            let result = backend.generate_3d_gaussian(params)
                .map_err(|e| ExecutorError::Inference(e))?;

            Ok(vec![
                json!({
                    "type": "gaussian_3d",
                    "file_path": output_path.to_string_lossy().to_string(),
                    "format": output_format,
                    "num_gaussians": result.num_gaussians,
                    "seed": seed,
                }),
                json!({
                    "type": "image",
                    "path": output_path.with_extension("").to_string_lossy().to_string() + "_preprocessed.webp",
                }),
            ])
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

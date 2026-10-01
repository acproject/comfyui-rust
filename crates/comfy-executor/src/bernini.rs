use crate::error::ExecutorError;
use crate::registry::NodeRegistry;
use comfy_core::{IoType, NodeClassDef, NodeInputTypes, InputTypeSpec};
use comfy_inference::image::{SdImage, SdVideo};
use comfy_inference::params::{BerniniVideoParams, ImageGenParams, ModelConfig};
use serde_json::{json, Value};
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

/// Output directory for Bernini-R videos (served via `/view?subfolder=videos`).
fn get_output_video_dir() -> PathBuf {
    let base = std::env::var("COMFY_OUTPUT_DIR").unwrap_or_else(|_| "output".to_string());
    let dir = PathBuf::from(&base).join("videos");
    if !dir.exists() {
        let _ = fs::create_dir_all(&dir);
    }
    dir
}

/// Resolve a path that may be absolute, relative to cwd, or relative to the
/// server `input/` directory (uploaded materials live there).
fn resolve_under_input(path: &str) -> String {
    let p = Path::new(path);
    if p.is_absolute() {
        return path.to_string();
    }
    if path.starts_with("input/") || p.exists() {
        return path.to_string();
    }
    let in_input = format!("input/{}", path);
    if Path::new(&in_input).exists() {
        in_input
    } else {
        path.to_string()
    }
}

/// Resolve the source video file carried by a VIDEO input value.
///
/// Understands LoadVideo's envelope
/// (`{"type":"video","videos":[{"filename":..,"subfolder":..,"type":"input"}]}`)
/// and a plain `{"path": ...}` reference.
fn resolve_input_video_path(value: &Value) -> Option<String> {
    if let Some(path) = value.get("path").and_then(|v| v.as_str()) {
        if !path.is_empty() {
            return Some(resolve_under_input(path));
        }
    }
    let entry = value
        .get("videos")
        .and_then(|v| v.as_array())
        .and_then(|a| a.first())?;
    let filename = entry.get("filename").and_then(|x| x.as_str())?;
    if filename.is_empty() {
        return None;
    }
    let rel = match entry.get("subfolder").and_then(|x| x.as_str()).unwrap_or("") {
        "" => filename.to_string(),
        sub => format!("{}/{}", sub, filename),
    };
    Some(resolve_under_input(&rel))
}

fn register_bernini_video_pipeline(registry: &mut NodeRegistry) {
    let model_choices: Vec<String> = scan_bernini_dirs().into_iter().map(|(n, _)| n).collect();
    let default_model = model_choices.first().cloned().unwrap_or_default();

    let class_def = NodeClassDef {
        class_type: "BerniniRVideoPipeline".to_string(),
        display_name: "Bernini-R 视频生成/编辑 (t2v/v2v)".to_string(),
        category: "video/bernini".to_string(),
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
                            Value::Array(model_choices.iter().map(|s| json!(s)).collect()));
                        e
                    },
                });
                m.insert("mode".to_string(), InputTypeSpec {
                    type_name: "COMBO".to_string(),
                    extra: {
                        let mut e = HashMap::new();
                        e.insert("choices".to_string(), json!(["auto", "t2v", "v2v"]));
                        e.insert("default".to_string(), json!("auto"));
                        e
                    },
                });
                m
            },
            optional: {
                let mut m = HashMap::new();
                m.insert("video".to_string(), InputTypeSpec {
                    type_name: "VIDEO".to_string(),
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
                m.insert("num_frames".to_string(), InputTypeSpec {
                    type_name: "INT".to_string(),
                    extra: {
                        let mut e = HashMap::new();
                        e.insert("min".to_string(), json!(9));
                        e.insert("max".to_string(), json!(129));
                        // Wan VAE temporal compression expects 4n+1 frames.
                        e.insert("step".to_string(), json!(4));
                        e.insert("default".to_string(), json!(81));
                        e
                    },
                });
                m.insert("fps".to_string(), InputTypeSpec {
                    type_name: "INT".to_string(),
                    extra: {
                        let mut e = HashMap::new();
                        e.insert("min".to_string(), json!(1));
                        e.insert("max".to_string(), json!(30));
                        e.insert("step".to_string(), json!(1));
                        e.insert("default".to_string(), json!(16));
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
                        e.insert("default".to_string(), json!(832));
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
                m.insert("max_image_size".to_string(), InputTypeSpec {
                    type_name: "INT".to_string(),
                    extra: {
                        let mut e = HashMap::new();
                        e.insert("min".to_string(), json!(256));
                        e.insert("max".to_string(), json!(1536));
                        e.insert("step".to_string(), json!(16));
                        e.insert("default".to_string(), json!(848));
                        e
                    },
                });
                m.insert("guidance_mode".to_string(), InputTypeSpec {
                    type_name: "COMBO".to_string(),
                    extra: {
                        let mut e = HashMap::new();
                        e.insert("choices".to_string(), json!(["auto", "t2v_apg", "v2v_apg"]));
                        e.insert("default".to_string(), json!("auto"));
                        e
                    },
                });
                m.insert("embed_frames".to_string(), InputTypeSpec {
                    type_name: "BOOLEAN".to_string(),
                    extra: {
                        let mut e = HashMap::new();
                        // Decode the mp4 into SdImage frames so SaveVideo /
                        // LTX / other VIDEO consumers can connect directly.
                        e.insert("default".to_string(), json!(true));
                        e
                    },
                });
                m
            },
            hidden: HashMap::new(),
        },
        output_types: vec![IoType::Video],
        output_names: vec!["VIDEO".to_string()],
        output_is_list: vec![false],
        is_output_node: false,
        has_intermediate_output: false,
        is_changed: None,
        not_idempotent: false,
        function_name: "run".to_string(),
    };

    registry.register(class_def, Arc::new(move |ctx, node, node_id| {
        let video_in = ctx.resolve_input(node_id, "video").unwrap_or_else(|_| json!(null));
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
        let mode = node.inputs.get("mode").and_then(|v| v.as_str()).unwrap_or("auto");
        let guidance_mode = node.inputs.get("guidance_mode")
            .and_then(|v| v.as_str())
            .unwrap_or("auto")
            .to_string();
        let seed = node.inputs.get("seed").and_then(|v| v.as_i64()).unwrap_or(42);
        let steps = node.inputs.get("steps").and_then(|v| v.as_i64()).unwrap_or(40) as i32;
        let num_frames = node.inputs.get("num_frames").and_then(|v| v.as_i64()).unwrap_or(81) as i32;
        let fps = node.inputs.get("fps").and_then(|v| v.as_i64()).unwrap_or(16) as i32;
        let width = node.inputs.get("width").and_then(|v| v.as_i64()).unwrap_or(832) as i32;
        let height = node.inputs.get("height").and_then(|v| v.as_i64()).unwrap_or(480) as i32;
        let max_image_size = node.inputs.get("max_image_size")
            .and_then(|v| v.as_i64()).unwrap_or(848) as i32;
        let embed_frames = node.inputs.get("embed_frames")
            .and_then(|v| v.as_bool())
            .unwrap_or(true);
        let node_id_str = node_id.to_string();

        Box::pin(async move {
            if prompt.trim().is_empty() {
                return Err(ExecutorError::NodeExecutionFailed {
                    node_id: node_id_str.clone(),
                    message: "BerniniRVideoPipeline requires a prompt".to_string(),
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

            // Resolve optional source clip (LoadVideo / {"path": ...}).
            let source_video: Option<String> = if video_in.is_null() {
                None
            } else {
                resolve_input_video_path(&video_in)
            };

            let task = match mode {
                "t2v" => "t2v",
                "v2v" => "v2v",
                _ => if source_video.is_some() { "v2v" } else { "t2v" },
            };
            if task == "v2v" {
                let src = source_video.as_deref().unwrap_or("");
                if src.is_empty() || !Path::new(src).exists() {
                    return Err(ExecutorError::NodeExecutionFailed {
                        node_id: node_id_str.clone(),
                        message: format!(
                            "v2v mode requires a VIDEO input; source not found: '{}'. \
                             Connect a LoadVideo node (upload the clip to input/ first).",
                            src
                        ),
                    });
                }
            }
            let effective_guidance = match guidance_mode.as_str() {
                "t2v_apg" | "v2v_apg" => guidance_mode.clone(),
                _ => if task == "v2v" { "v2v_apg" } else { "t2v_apg" }.to_string(),
            };

            let output_dir = get_output_video_dir();
            let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S");
            let filename = format!("bernini_{}_{}.mp4", timestamp, seed);
            let output_path = output_dir.join(&filename);

            let mut params = BerniniVideoParams::new(
                ModelConfig::new().with_model(model_dir.to_string_lossy().to_string()),
                prompt,
            )
            .with_seed(seed)
            .with_steps(steps)
            .with_num_frames(num_frames)
            .with_fps(fps)
            .with_dimensions(width, height)
            .with_max_image_size(max_image_size)
            .with_guidance_mode(effective_guidance)
            .with_output_path(output_path.to_string_lossy().to_string());
            if !negative_prompt.is_empty() {
                params = params.with_negative_prompt(negative_prompt);
            }
            if task == "v2v" {
                params = params.with_input_video(source_video.unwrap());
            }

            let backend = ctx.backend();
            let result = backend.generate_bernini_video(params)
                .map_err(ExecutorError::Inference)?;

            // File-reference envelope (frontend fetches the mp4 via /view).
            let mut envelope = json!({
                "type": "video",
                "videos": [{
                    "filename": filename,
                    "subfolder": "videos",
                    "type": "output",
                    "frame_count": result.num_frames,
                    "fps": result.fps,
                }],
                "path": result.output_file,
                "filename": filename,
                "subfolder": "videos",
                "frame_count": result.num_frames,
                "fps": result.fps,
                "task": result.task,
                "guidance_mode": result.guidance_mode,
            });

            // Also expose decoded frames so frame-based VIDEO nodes
            // (SaveVideo, VideoVAEDecode consumers, LTX chains) connect too.
            if embed_frames {
                if SdVideo::is_ffmpeg_available() {
                    match SdVideo::decode_with_ffmpeg(Path::new(&result.output_file), result.fps) {
                        Ok(video) => {
                            let frames: Vec<Value> = video.frames.iter()
                                .map(|f| serde_json::to_value(f).unwrap_or(json!({})))
                                .collect();
                            envelope["frames"] = Value::Array(frames);
                            tracing::info!(
                                "BerniniRVideoPipeline: embedded {} frames for VIDEO consumers",
                                video.frames.len()
                            );
                        }
                        Err(e) => tracing::warn!(
                            "BerniniRVideoPipeline: mp4 written but frame decoding failed: {} \
                             (file reference is still available via /view)",
                            e
                        ),
                    }
                } else {
                    tracing::warn!(
                        "BerniniRVideoPipeline: ffmpeg unavailable; skipping embedded frames \
                         (mp4 file reference still available via /view)"
                    );
                }
            }

            Ok(vec![envelope])
        })
    }));
}

pub fn register_bernini_nodes(registry: &mut NodeRegistry) {
    register_bernini_pipeline(registry);
    register_bernini_video_pipeline(registry);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mkdirs(root: &Path, name: &str) {
        let dir = root.join(name);
        for sub in ["transformer", "vae", "tokenizer", "text_encoder"] {
            fs::create_dir_all(dir.join(sub)).unwrap();
        }
        fs::write(dir.join("config.json"), b"{}").unwrap();
    }

    #[test]
    fn scans_only_self_contained_bernini_dirs() {
        let _env = crate::TEST_ENV_LOCK.lock().unwrap();
        let root = std::env::temp_dir().join(format!("comfy_bernini_scan_test_{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();

        // Usable: self-contained Bernini-R layout at the models root.
        mkdirs(&root, "Bernini-R-1.3B-Diffusers");
        // Usable: nested under diffusion_models/.
        mkdirs(&root.join("diffusion_models"), "bernini-r-diffusers");
        // Ignored: name does not mention bernini.
        mkdirs(&root, "wan21-base");
        // Ignored: bernini in name but missing components (separate renderer).
        let incomplete = root.join("Bernini-R");
        fs::create_dir_all(&incomplete).unwrap();
        fs::write(incomplete.join("config.json"), b"{}").unwrap();

        unsafe { std::env::set_var("COMFY_MODELS_DIR", &root); }
        let found: Vec<String> = scan_bernini_dirs().into_iter().map(|(n, _)| n).collect();
        assert_eq!(found, vec!["Bernini-R-1.3B-Diffusers", "bernini-r-diffusers"]);

        let resolved = resolve_bernini_dir("Bernini-R-1.3B-Diffusers").unwrap();
        assert!(is_self_contained_bernini_dir(&resolved));
        assert!(resolve_bernini_dir("Bernini-R").is_none());

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn resolves_loadvideo_envelope_and_paths() {
        // LoadVideo envelope: filename relative to input/.
        let env = json!({
            "type": "video",
            "videos": [{"filename": "clips/a.mp4", "subfolder": "", "type": "input"}],
            "fps": 16,
        });
        assert_eq!(resolve_input_video_path(&env).unwrap(), "clips/a.mp4");

        // Subfolder is joined (defensive; LoadVideo normally scans recursively).
        let env = json!({
            "videos": [{"filename": "a.mp4", "subfolder": "clips", "type": "input"}],
        });
        assert_eq!(resolve_input_video_path(&env).unwrap(), "clips/a.mp4");

        // Absolute {"path":..} wins as-is.
        let env = json!({"path": "/tmp/clip.mp4"});
        assert_eq!(resolve_input_video_path(&env).unwrap(), "/tmp/clip.mp4");

        // Empty / malformed values.
        assert!(resolve_input_video_path(&json!(null)).is_none());
        assert!(resolve_input_video_path(&json!({"videos": []})).is_none());
        assert!(resolve_input_video_path(&json!({"path": ""})).is_none());
    }

    #[test]
    fn video_node_is_registered_in_builtin_registry() {
        let _env = crate::TEST_ENV_LOCK.lock().unwrap();
        // Point at the real models tree when it exists so the combo gets the
        // self-contained Bernini-R directory; otherwise just verify wiring.
        let real_models = Path::new("/home/acproject/usb/comfyui/models");
        if real_models.exists() {
            unsafe { std::env::set_var("COMFY_MODELS_DIR", real_models); }
        }
        let mut registry = NodeRegistry::new();
        crate::builtin_nodes::register_builtin_nodes(&mut registry);

        let def = registry
            .get_class_def("BerniniRVideoPipeline")
            .expect("BerniniRVideoPipeline must be registered");
        assert_eq!(def.output_types, vec![IoType::Video]);
        assert_eq!(def.output_names, vec!["VIDEO".to_string()]);
        assert!(def.input_types.required.contains_key("prompt"));
        assert!(def.input_types.required.contains_key("mode"));
        assert!(def.input_types.optional.contains_key("video"));
        assert!(def.input_types.optional.contains_key("num_frames"));
        assert!(def.input_types.optional.contains_key("embed_frames"));

        // Image node must remain registered alongside the video node.
        assert!(registry.get_class_def("BerniniRPipeline").is_some());

        if real_models.exists() {
            let choices = def.input_types.required.get("bernini_model")
                .and_then(|s| s.extra.get("choices"))
                .and_then(|v| v.as_array())
                .expect("model choices");
            let labels: Vec<&str> = choices.iter().filter_map(|c| c.as_str()).collect();
            assert!(
                labels.iter().any(|l| l.to_lowercase().contains("bernini")),
                "self-contained Bernini dir should appear in combo: {labels:?}"
            );
        }
    }

    #[test]
    fn resolves_relative_video_under_input_dir() {
        let _env = crate::TEST_ENV_LOCK.lock().unwrap();
        let root = std::env::temp_dir().join(format!("comfy_bernini_input_test_{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        let cwd = std::env::current_dir().unwrap();
        let input_dir = root.join("input");
        fs::create_dir_all(&input_dir.join("clips")).unwrap();
        let file = input_dir.join("clips").join("b.mp4");
        fs::write(&file, b"x").unwrap();

        // Run inside the temp root so the relative input/ lookup hits.
        assert!(std::env::set_current_dir(&root).is_ok());
        assert_eq!(resolve_under_input("clips/b.mp4"), "input/clips/b.mp4");
        assert_eq!(
            resolve_input_video_path(&json!({"videos":[{"filename":"clips/b.mp4"}]})).unwrap(),
            "input/clips/b.mp4"
        );
        let _ = std::env::set_current_dir(&cwd);
        let _ = fs::remove_dir_all(&root);
    }
}

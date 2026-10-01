//! MiniMax 控制节点：
//! - `MiniMaxH3ContextIR`：文本/图像/视频 -> 结构化 H3 上下文（托管 MiniMax
//!   API，或无 key 时的本地模板降级）
//! - `MiniMaxH3Pipeline`：MiniMax-H3 全模态音视频生成（t2va / i2va / ref2va，
//!   通过 py/flash_attn_v100/comfy_fallback/h3_generate.py 兜底）
//! - `MiniMaxMusic3`：MiniMax-Music3 文生音乐（lyrics + structured caption，
//!   通过 music3_generate.py 兜底）

use crate::error::ExecutorError;
use crate::registry::NodeRegistry;
use comfy_core::{InputTypeSpec, IoType, NodeClassDef, NodeInputTypes};
use comfy_inference::image::{SdImage, SdVideo};
use comfy_inference::params::{
    ContextIrParams, H3Context, H3Mode, H3Params, Music3Params,
};
use serde_json::{json, Value};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

fn get_models_dir() -> String {
    std::env::var("COMFY_MODELS_DIR").unwrap_or_else(|_| "models".to_string())
}

fn get_output_dir(sub: &str) -> PathBuf {
    let base = std::env::var("COMFY_OUTPUT_DIR").unwrap_or_else(|_| "output".to_string());
    let dir = PathBuf::from(&base).join(sub);
    if !dir.exists() {
        let _ = fs::create_dir_all(&dir);
    }
    dir
}

/// Read `_class_name` from a MiniMax modular repository.
fn modular_class_name(dir: &Path) -> Option<String> {
    let text = fs::read_to_string(dir.join("modular_model_index.json")).ok()?;
    let v: Value = serde_json::from_str(&text).ok()?;
    v.get("_class_name").and_then(|x| x.as_str()).map(str::to_string)
}

/// Scan the models roots for directories whose modular index class matches
/// `class_marker` (e.g. "MiniMaxH3", "Music3").
fn scan_model_dirs(class_marker: &str) -> Vec<(String, PathBuf)> {
    let base = PathBuf::from(get_models_dir());
    let mut roots = vec![base.clone(), base.join("diffusion_models")];
    if let Ok(cwd) = std::env::current_dir() {
        roots.push(cwd.join(&base));
    }
    let mut found: Vec<(String, PathBuf)> = Vec::new();
    let mut seen = HashSet::new();
    for root in roots {
        let Ok(entries) = fs::read_dir(&root) else { continue };
        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }
            let marker_hit = modular_class_name(&path)
                .map(|c| c.contains(class_marker))
                .unwrap_or(false);
            if marker_hit {
                let key = path.to_string_lossy().to_string();
                if seen.insert(key) {
                    found.push((path.file_name().unwrap().to_string_lossy().to_string(), path));
                }
            }
        }
    }
    found.sort_by(|a, b| a.0.cmp(&b.0));
    found
}

fn resolve_model_dir(choices: &[(String, PathBuf)], label: &str) -> Option<PathBuf> {
    choices.iter().find(|(n, _)| n == label).map(|(_, p)| p.clone())
}

/// Resolve an IMAGE input value (LoadImage `{path}` / `{images:[…]}` envelope
/// or an inline serialized `SdImage`) into decoded images.
fn resolve_input_images(value: &Value, node_id: &str) -> Result<Vec<SdImage>, ExecutorError> {
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
        SdImage::from_png_bytes(&bytes).map_err(|e| ExecutorError::NodeExecutionFailed {
            node_id: node_id.to_string(),
            message: format!("Failed to decode image '{}': {}", resolved, e),
        })
    };

    if value.is_null() {
        return Ok(Vec::new());
    }
    // {path: "..."} / {images: [{...}, ...]}
    if let Some(obj) = value.as_object() {
        if let Some(path) = obj.get("path").and_then(|v| v.as_str()) {
            if !path.is_empty() {
                return Ok(vec![decode_path(path)?]);
            }
        }
        if let Some(arr) = obj.get("images").and_then(|v| v.as_array()) {
            let mut out = Vec::new();
            for item in arr {
                if let Some(path) = item.get("path").and_then(|v| v.as_str()) {
                    if !path.is_empty() {
                        out.push(decode_path(path)?);
                        continue;
                    }
                }
                if let Ok(img) = serde_json::from_value::<SdImage>(item.clone()) {
                    out.push(img);
                }
            }
            return Ok(out);
        }
    }
    // Inline serialized SdImage.
    match serde_json::from_value::<SdImage>(value.clone()) {
        Ok(img) => Ok(vec![img]),
        Err(_) => Ok(Vec::new()),
    }
}

// ---------------------------------------------------------------------------
// MiniMaxH3ContextIR
// ---------------------------------------------------------------------------

fn register_h3_context_ir(registry: &mut NodeRegistry) {
    let class_def = NodeClassDef {
        class_type: "MiniMaxH3ContextIR".to_string(),
        display_name: "MiniMax-H3 Context-IR（多模态上下文解析）".to_string(),
        category: "minimax/h3".to_string(),
        input_types: NodeInputTypes {
            required: {
                let mut m = HashMap::new();
                m.insert("text_prompt".to_string(), string_input(true, ""));
                m
            },
            optional: {
                let mut m = HashMap::new();
                m.insert("image".to_string(), InputTypeSpec {
                    type_name: "IMAGE".to_string(),
                    extra: HashMap::new(),
                });
                m.insert("video".to_string(), InputTypeSpec {
                    type_name: "VIDEO".to_string(),
                    extra: HashMap::new(),
                });
                m.insert("parse_sfx".to_string(), bool_input(true));
                m.insert("parse_bgm".to_string(), bool_input(false));
                m
            },
            hidden: HashMap::new(),
        },
        output_types: vec![IoType::H3Context, IoType::String],
        output_names: vec!["H3_CONTEXT".to_string(), "formatted_prompt".to_string()],
        output_is_list: vec![false, false],
        is_output_node: false,
        has_intermediate_output: false,
        is_changed: None,
        not_idempotent: true,
        function_name: "parse_context".to_string(),
    };

    registry.register(class_def, Arc::new(|ctx, node, node_id| {
        let text_prompt = node.inputs.get("text_prompt")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let parse_sfx = node.inputs.get("parse_sfx")
            .and_then(|v| v.as_bool())
            .unwrap_or(true);
        let parse_bgm = node.inputs.get("parse_bgm")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let has_image = ctx.resolve_input(node_id, "image")
            .map(|v| !v.is_null())
            .unwrap_or(false);
        let has_video = ctx.resolve_input(node_id, "video")
            .map(|v| !v.is_null())
            .unwrap_or(false);
        let node_id_str = node_id.to_string();

        Box::pin(async move {
            if text_prompt.trim().is_empty() && !has_image && !has_video {
                return Err(ExecutorError::NodeExecutionFailed {
                    node_id: node_id_str,
                    message: "MiniMaxH3ContextIR requires a text prompt or an image/video input"
                        .to_string(),
                });
            }
            let mut params = ContextIrParams::from_text(text_prompt);
            params.parse_sfx = parse_sfx;
            params.parse_bgm = parse_bgm;

            let context: H3Context = ctx.backend()
                .context_ir(params)
                .map_err(ExecutorError::Inference)?;
            let formatted = context.build_positive_prompt();
            let context_val = serde_json::to_value(&context).map_err(|e| {
                ExecutorError::NodeExecutionFailed {
                    node_id: node_id.to_string(),
                    message: format!("Failed to serialize H3 context: {e}"),
                }
            })?;
            Ok(vec![context_val, json!(formatted)])
        })
    }));
}

// ---------------------------------------------------------------------------
// MiniMaxH3Pipeline
// ---------------------------------------------------------------------------

fn register_h3_pipeline(registry: &mut NodeRegistry) {
    let choices = scan_model_dirs("MiniMaxH3");
    let default_model = choices.first().map(|(n, _)| n.clone()).unwrap_or_default();
    let model_labels: Vec<Value> = choices.iter().map(|(n, _)| json!(n)).collect();

    let class_def = NodeClassDef {
        class_type: "MiniMaxH3Pipeline".to_string(),
        display_name: "MiniMax-H3 全模态音视频生成 (t2va/i2va/ref2va)".to_string(),
        category: "minimax/h3".to_string(),
        input_types: NodeInputTypes {
            required: {
                let mut m = HashMap::new();
                m.insert("prompt".to_string(), string_input(true, ""));
                m.insert("minimax_h3_model".to_string(), combo_input(model_labels.clone()));
                m.insert("mode".to_string(), combo_with_default(
                    json!(["auto", "t2va", "i2va", "ref2va"]), "auto"));
                m
            },
            optional: {
                let mut m = HashMap::new();
                m.insert("context".to_string(), InputTypeSpec {
                    type_name: "H3_CONTEXT".to_string(),
                    extra: HashMap::new(),
                });
                m.insert("image".to_string(), InputTypeSpec {
                    type_name: "IMAGE".to_string(),
                    extra: HashMap::new(),
                });
                m.insert("seed".to_string(), int_input(0, u32::MAX as i64, 1, 42));
                m.insert("steps".to_string(), int_input(1, 100, 1, 30));
                // 17n+5 对齐，范围 123 (≈5s) – 362 (≈15s)。
                m.insert("num_frames".to_string(), int_input(123, 362, 17, 123));
                // 0 = 由 pipeline 自动决定（短边 768，按宽高比）。
                m.insert("width".to_string(), int_input(0, 1536, 32, 0));
                m.insert("height".to_string(), int_input(0, 1536, 32, 0));
                m.insert("embed_frames".to_string(), bool_input(true));
                m
            },
            hidden: HashMap::new(),
        },
        output_types: vec![IoType::Video, IoType::Audio],
        output_names: vec!["VIDEO".to_string(), "AUDIO".to_string()],
        output_is_list: vec![false, false],
        is_output_node: false,
        has_intermediate_output: false,
        is_changed: None,
        not_idempotent: true,
        function_name: "generate".to_string(),
    };

    registry.register(class_def, Arc::new(move |ctx, node, node_id| {
        let image_in = ctx.resolve_input(node_id, "image").unwrap_or_else(|_| json!(null));
        let context_in = ctx.resolve_input(node_id, "context").unwrap_or_else(|_| json!(null));
        let model_label = node.inputs.get("minimax_h3_model")
            .and_then(|v| v.as_str())
            .map(str::to_string)
            .unwrap_or_else(|| default_model.clone());
        let mode = node.inputs.get("mode")
            .and_then(|v| v.as_str())
            .unwrap_or("auto")
            .to_string();
        let prompt = node.inputs.get("prompt")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let seed = node.inputs.get("seed").and_then(|v| v.as_i64()).unwrap_or(42);
        let steps = node.inputs.get("steps").and_then(|v| v.as_i64()).unwrap_or(30) as i32;
        let num_frames = node.inputs.get("num_frames").and_then(|v| v.as_i64()).unwrap_or(123) as i32;
        let width = node.inputs.get("width").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
        let height = node.inputs.get("height").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
        let embed_frames = node.inputs.get("embed_frames")
            .and_then(|v| v.as_bool())
            .unwrap_or(true);
        let node_id_str = node_id.to_string();

        Box::pin(async move {
            if prompt.trim().is_empty() && context_in.is_null() {
                return Err(ExecutorError::NodeExecutionFailed {
                    node_id: node_id_str.clone(),
                    message: "MiniMaxH3Pipeline requires a prompt (or a H3_CONTEXT input)"
                        .to_string(),
                });
            }
            let model_dir = resolve_model_dir(&scan_model_dirs("MiniMaxH3"), &model_label)
                .ok_or_else(|| ExecutorError::NodeExecutionFailed {
                    node_id: node_id_str.clone(),
                    message: format!(
                        "MiniMax-H3 model directory '{}' not found under COMFY_MODELS_DIR \
                         (need a directory with modular_model_index.json _class_name=MiniMaxH3*)",
                        model_label
                    ),
                })?;

            let mut images = resolve_input_images(&image_in, &node_id_str)?;

            // auto：有图走 i2va(fl2va)，无图走 t2va。
            let (h3_mode, variant) = match mode.as_str() {
                "t2va" => (H3Mode::T2VA, "fl2va".to_string()),
                "i2va" => {
                    if images.is_empty() {
                        return Err(ExecutorError::NodeExecutionFailed {
                            node_id: node_id_str.clone(),
                            message: "i2va mode requires an IMAGE input (first keyframe)".to_string(),
                        });
                    }
                    (H3Mode::I2VA, "fl2va".to_string())
                }
                "ref2va" => {
                    if images.is_empty() {
                        return Err(ExecutorError::NodeExecutionFailed {
                            node_id: node_id_str.clone(),
                            message: "ref2va mode requires at least one reference IMAGE".to_string(),
                        });
                    }
                    (H3Mode::Ref2VA, "ref2va".to_string())
                }
                _ => {
                    if images.is_empty() {
                        (H3Mode::T2VA, "fl2va".to_string())
                    } else {
                        (H3Mode::I2VA, "fl2va".to_string())
                    }
                }
            };

            // H3_CONTEXT -> 结构化提示词；用户 prompt 作为补充细节追加。
            let mut final_prompt = prompt.clone();
            let mut h3_context: Option<H3Context> = None;
            if let Some(obj) = context_in.as_object() {
                if let Ok(parsed) = serde_json::from_value::<H3Context>(Value::Object(obj.clone())) {
                    let formatted = parsed.build_positive_prompt();
                    final_prompt = if prompt.trim().is_empty() {
                        formatted
                    } else {
                        format!("{}. {}", formatted, prompt)
                    };
                    h3_context = Some(parsed);
                }
            }
            if final_prompt.trim().is_empty() {
                return Err(ExecutorError::NodeExecutionFailed {
                    node_id: node_id_str.clone(),
                    message: "MiniMaxH3Pipeline resolved an empty prompt".to_string(),
                });
            }

            let mut params = H3Params::new(final_prompt)
                .with_model_path(model_dir.to_string_lossy().to_string())
                .with_variant(variant)
                .with_steps(steps)
                .with_num_frames(num_frames)
                .with_seed(seed);
            params.mode = h3_mode;
            if width > 0 && height > 0 {
                params = params.with_resolution(width, height);
            } else {
                // 0 = 自动：清掉 H3Params 默认分辨率，让脚本走短边/宽高比逻辑。
                params.width = 0;
                params.height = 0;
            }
            if !images.is_empty() {
                // i2va 只取首帧；ref2va 传入全部参考图。
                if matches!(h3_mode, H3Mode::I2VA) {
                    params.reference_images = vec![images.remove(0)];
                } else {
                    params.reference_images = std::mem::take(&mut images);
                }
            }
            if let Some(c) = h3_context {
                params = params.with_context(c);
            }

            let video: SdVideo = ctx.backend()
                .generate_av(params)
                .map_err(ExecutorError::Inference)?;

            // 落盘到 output/videos/。
            let video_dir = get_output_dir("videos");
            let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S");
            let filename = format!("minimax_h3_{}_{}.mp4", timestamp, seed);
            let dest_mp4 = video_dir.join(&filename);
            let tmp_mp4 = std::env::temp_dir()
                .join("comfyui-rust")
                .join(format!("py_h3_{}.mp4", seed));
            let src_mp4 = if dest_mp4.exists() {
                dest_mp4.clone()
            } else if tmp_mp4.exists() {
                fs::copy(&tmp_mp4, &dest_mp4).map_err(|e| ExecutorError::NodeExecutionFailed {
                    node_id: node_id_str.clone(),
                    message: format!("Failed to stage H3 mp4: {e}"),
                })?;
                dest_mp4.clone()
            } else {
                return Err(ExecutorError::NodeExecutionFailed {
                    node_id: node_id_str,
                    message: "H3 backend returned no mp4 file".to_string(),
                });
            };

            let frame_count = video.frame_count();
            let mut envelope = json!({
                "type": "video",
                "videos": [{
                    "filename": filename,
                    "subfolder": "videos",
                    "type": "output",
                    "frame_count": frame_count,
                    "fps": video.fps,
                }],
                "path": src_mp4.to_string_lossy().to_string(),
                "filename": filename,
                "subfolder": "videos",
                "frame_count": frame_count,
                "fps": video.fps,
            });
            if embed_frames {
                let frames: Vec<Value> = video.frames.iter()
                    .map(|f| serde_json::to_value(f).unwrap_or(json!({})))
                    .collect();
                envelope["frames"] = Value::Array(frames);
            }

            // 副输出 AUDIO：写 sidecar wav 以便 SaveAudio 等节点直连。
            let audio_envelope = match video.audio {
                Some(audio) => {
                    let audio_dir = get_output_dir("audio");
                    let wav_name = format!("minimax_h3_{}_{}.wav", timestamp, seed);
                    let wav_path = audio_dir.join(&wav_name);
                    fs::write(&wav_path, audio.to_wav_bytes()).map_err(|e| {
                        ExecutorError::NodeExecutionFailed {
                            node_id: node_id_str.clone(),
                            message: format!("Failed to write H3 sidecar wav: {e}"),
                        }
                    })?;
                    if let Ok(a) = serde_json::to_value(&audio) {
                        envelope["audio"] = a;
                    }
                    json!({
                        "type": "audio",
                        "path": wav_path.to_string_lossy().to_string(),
                        "filename": wav_name,
                        "subfolder": "audio",
                        "format": "wav",
                        "duration": audio.duration_sec(),
                        "sample_rate": audio.sample_rate,
                        "channels": audio.channels,
                        "audios": [{
                            "filename": wav_name,
                            "subfolder": "audio",
                            "type": "output",
                        }],
                    })
                }
                None => json!({
                    "type": "audio",
                    "path": "",
                    "audios": [],
                    "empty": true,
                }),
            };

            Ok(vec![envelope, audio_envelope])
        })
    }));
}

// ---------------------------------------------------------------------------
// MiniMaxMusic3
// ---------------------------------------------------------------------------

fn register_music3(registry: &mut NodeRegistry) {
    let choices = scan_model_dirs("Music3");
    let default_model = choices.first().map(|(n, _)| n.clone()).unwrap_or_default();
    let model_labels: Vec<Value> = choices.iter().map(|(n, _)| json!(n)).collect();

    let class_def = NodeClassDef {
        class_type: "MiniMaxMusic3".to_string(),
        display_name: "MiniMax-Music3 文生音乐 (lyrics + caption)".to_string(),
        category: "minimax/music3".to_string(),
        input_types: NodeInputTypes {
            required: {
                let mut m = HashMap::new();
                m.insert("minimax_music3_model".to_string(), combo_input(model_labels.clone()));
                m.insert("prompt".to_string(), string_input(true, ""));
                m
            },
            optional: {
                let mut m = HashMap::new();
                m.insert("lyrics".to_string(), string_input(true, ""));
                m.insert("audio_duration".to_string(), float_input(1.0, 360.0, 1.0, 60.0));
                m.insert("seed".to_string(), int_input(0, u32::MAX as i64, 1, 0));
                // 0 = 按 audio_duration*25 自动推导（硬上限 9000 ≈ 360s）。
                m.insert("max_new_tokens".to_string(), int_input(0, 9000, 1, 0));
                m
            },
            hidden: HashMap::new(),
        },
        output_types: vec![IoType::Audio],
        output_names: vec!["AUDIO".to_string()],
        output_is_list: vec![false],
        is_output_node: true,
        has_intermediate_output: false,
        is_changed: None,
        not_idempotent: true,
        function_name: "generate".to_string(),
    };

    registry.register(class_def, Arc::new(move |ctx, node, node_id| {
        let model_label = node.inputs.get("minimax_music3_model")
            .and_then(|v| v.as_str())
            .map(str::to_string)
            .unwrap_or_else(|| default_model.clone());
        let prompt = node.inputs.get("prompt")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let lyrics = node.inputs.get("lyrics")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let audio_duration = node.inputs.get("audio_duration")
            .and_then(|v| v.as_f64())
            .unwrap_or(60.0);
        let seed = node.inputs.get("seed").and_then(|v| v.as_i64()).unwrap_or(0);
        let max_new_tokens = node.inputs.get("max_new_tokens")
            .and_then(|v| v.as_i64())
            .unwrap_or(0) as i32;
        let node_id_str = node_id.to_string();

        Box::pin(async move {
            if prompt.trim().is_empty() {
                return Err(ExecutorError::NodeExecutionFailed {
                    node_id: node_id_str,
                    message: "MiniMaxMusic3 requires a structured caption in 'prompt'".to_string(),
                });
            }
            let model_dir = resolve_model_dir(&scan_model_dirs("Music3"), &model_label)
                .ok_or_else(|| ExecutorError::NodeExecutionFailed {
                    node_id: node_id_str.clone(),
                    message: format!(
                        "MiniMax-Music3 directory '{}' not found under COMFY_MODELS_DIR \
                         (need modular_model_index.json _class_name=*Music3*)",
                        model_label
                    ),
                })?;

            let audio_dir = get_output_dir("audio");
            let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S");
            let filename = format!("minimax_music3_{}_{}.wav", timestamp, seed);
            let output_path = audio_dir.join(&filename);

            let mut params = Music3Params::new(prompt)
                .with_model_path(model_dir.to_string_lossy().to_string())
                .with_lyrics(lyrics)
                .with_duration(audio_duration)
                .with_seed(seed)
                .with_output_path(output_path.to_string_lossy().to_string());
            if max_new_tokens > 0 {
                params = params.with_max_new_tokens(max_new_tokens);
            }

            let result = ctx.backend()
                .generate_music3(params)
                .map_err(ExecutorError::Inference)?;

            // 保持 SdAudio 内联，方便非文件型 AUDIO 消费者。
            let audio_val = serde_json::to_value(&result.audio).unwrap_or(json!({}));
            Ok(vec![json!({
                "type": "audio",
                "path": result.output_file,
                "filename": filename,
                "subfolder": "audio",
                "format": "wav",
                "duration": result.duration_sec,
                "sample_rate": result.sample_rate,
                "channels": result.channels,
                "seed": result.seed,
                "audio": audio_val,
                "audios": [{
                    "filename": filename,
                    "subfolder": "audio",
                    "type": "output",
                }],
            })])
        })
    }));
}

// ---------------------------------------------------------------------------
// input spec helpers
// ---------------------------------------------------------------------------

fn string_input(multiline: bool, default: &str) -> InputTypeSpec {
    let mut extra = HashMap::new();
    if multiline {
        extra.insert("multiline".to_string(), json!(true));
    }
    extra.insert("default".to_string(), json!(default));
    InputTypeSpec {
        type_name: "STRING".to_string(),
        extra,
    }
}

fn bool_input(default: bool) -> InputTypeSpec {
    let mut extra = HashMap::new();
    extra.insert("default".to_string(), json!(default));
    InputTypeSpec {
        type_name: "BOOLEAN".to_string(),
        extra,
    }
}

fn int_input(min: i64, max: i64, step: i64, default: i64) -> InputTypeSpec {
    let mut extra = HashMap::new();
    extra.insert("min".to_string(), json!(min));
    extra.insert("max".to_string(), json!(max));
    extra.insert("step".to_string(), json!(step));
    extra.insert("default".to_string(), json!(default));
    InputTypeSpec {
        type_name: "INT".to_string(),
        extra,
    }
}

fn float_input(min: f64, max: f64, step: f64, default: f64) -> InputTypeSpec {
    let mut extra = HashMap::new();
    extra.insert("min".to_string(), json!(min));
    extra.insert("max".to_string(), json!(max));
    extra.insert("step".to_string(), json!(step));
    extra.insert("default".to_string(), json!(default));
    InputTypeSpec {
        type_name: "FLOAT".to_string(),
        extra,
    }
}

fn combo_input(choices: Vec<Value>) -> InputTypeSpec {
    let mut extra = HashMap::new();
    extra.insert("choices".to_string(), Value::Array(choices));
    InputTypeSpec {
        type_name: "COMBO".to_string(),
        extra,
    }
}

fn combo_with_default(choices: Value, default: &str) -> InputTypeSpec {
    let mut extra = HashMap::new();
    extra.insert("choices".to_string(), choices);
    extra.insert("default".to_string(), json!(default));
    InputTypeSpec {
        type_name: "COMBO".to_string(),
        extra,
    }
}

pub fn register_minimax_nodes(registry: &mut NodeRegistry) {
    register_h3_context_ir(registry);
    register_h3_pipeline(registry);
    register_music3(registry);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write_modular(root: &Path, name: &str, class: &str) {
        let dir = root.join(name);
        fs::create_dir_all(&dir).unwrap();
        fs::write(
            dir.join("modular_model_index.json"),
            format!("{{\"_class_name\":\"{}\"}}", class),
        )
        .unwrap();
    }

    #[test]
    fn scans_h3_and_music3_dirs_separately() {
        let _env = crate::TEST_ENV_LOCK.lock().unwrap();
        let root = std::env::temp_dir()
            .join(format!("comfy_minimax_scan_test_{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        write_modular(&root, "MiniMax-H3", "MiniMaxH3ModularPipeline");
        write_modular(&root, "MiniMax-Music3", "MiniMaxMusic3ModularPipeline");
        write_modular(&root, "SomeOther", "SomeOtherPipeline");

        unsafe { std::env::set_var("COMFY_MODELS_DIR", &root); }

        let h3 = scan_model_dirs("MiniMaxH3");
        assert_eq!(h3.len(), 1);
        assert_eq!(h3[0].0, "MiniMax-H3");

        let music = scan_model_dirs("Music3");
        assert_eq!(music.len(), 1);
        assert_eq!(music[0].0, "MiniMax-Music3");

        assert!(resolve_model_dir(&h3, "MiniMax-H3").is_some());
        assert!(resolve_model_dir(&h3, "Missing").is_none());

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn decodes_image_envelope_shapes() {
        let _env = crate::TEST_ENV_LOCK.lock().unwrap();
        let root = std::env::temp_dir()
            .join(format!("comfy_minimax_img_test_{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        let cwd = std::env::current_dir().unwrap();
        assert!(std::env::set_current_dir(&root).is_ok());

        // Null / empty -> no images.
        assert!(resolve_input_images(&json!(null), "n").unwrap().is_empty());

        // Inline serialized SdImage (1x1 RGB black).
        let img = SdImage::new(8, 8, 3);
        let v = serde_json::to_value(&img).unwrap();
        assert_eq!(resolve_input_images(&v, "n").unwrap().len(), 1);

        let _ = std::env::set_current_dir(cwd);
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn minimax_nodes_registered() {
        let mut registry = NodeRegistry::new();
        crate::builtin_nodes::register_builtin_nodes(&mut registry);
        assert!(registry.get_class_def("MiniMaxH3ContextIR").is_some());
        assert!(registry.get_class_def("MiniMaxH3Pipeline").is_some());
        assert!(registry.get_class_def("MiniMaxMusic3").is_some());

        let h3 = registry.get_class_def("MiniMaxH3Pipeline").unwrap();
        assert_eq!(h3.output_types, vec![IoType::Video, IoType::Audio]);
        assert!(h3.input_types.required.contains_key("minimax_h3_model"));
        assert!(h3.input_types.optional.contains_key("context"));
        assert!(h3.input_types.optional.contains_key("num_frames"));

        let ir = registry.get_class_def("MiniMaxH3ContextIR").unwrap();
        assert_eq!(ir.output_types, vec![IoType::H3Context, IoType::String]);

        let m3 = registry.get_class_def("MiniMaxMusic3").unwrap();
        assert_eq!(m3.output_types, vec![IoType::Audio]);
        assert!(m3.input_types.optional.contains_key("audio_duration"));
    }
}

//! MiniMax-H3 Ref2VA (RefMod) nodes for ComfyUI-Rust.
//!
//! Implements the no-training reference-mod workflow from
//! ``ComfyUI-MiniMaxH3Mod`` as native Rust nodes that call a Python fallback
//! script for H3 VAE encode/decode.
//!
//! Nodes:
//!   * ``MiniMaxH3RefModExtract``   — encode image(s) → .safetensors RefMod
//!   * ``MiniMaxH3RefModsLoader``  — load 1-8 saved mods with strength/copies
//!   * ``MiniMaxH3RefModFolderLoader`` — list images/videos in a folder
//!   * ``MiniMaxH3RefModApply``    — decode mods → reference images on H3 cond
//!   * ``MiniMaxH3RefModsAxis``    — A/B mod pairs on signed sliders

use crate::error::ExecutorError;
use crate::registry::NodeRegistry;
use comfy_core::{InputTypeSpec, IoType, NodeClassDef, NodeInputTypes};
use comfy_inference::image::SdImage;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

const REFMODS_IO: &str = "H3_REF_MODS";

/// Resolve ``models/refmods/`` under the workspace root.
fn refmods_dir() -> PathBuf {
    if let Ok(dir) = std::env::var("COMFY_REFMODS_DIR") {
        let p = PathBuf::from(dir);
        if p.exists() || p.parent().map(|x| x.exists()).unwrap_or(false) {
            std::fs::create_dir_all(&p).ok();
            return p;
        }
    }
    for cand in [
        std::env::current_dir().ok().map(|d| d.join("models/refmods")),
        std::env::current_exe().ok().and_then(|p| {
            p.ancestors().nth(3).map(|r| r.join("models/refmods"))
        }),
    ].into_iter().flatten() {
        if cand.exists() || cand.parent().map(|x| x.exists()).unwrap_or(false) {
            std::fs::create_dir_all(&cand).ok();
            return cand;
        }
    }
    let dir = std::env::current_dir()
        .unwrap_or_else(|_| PathBuf::from("."))
        .join("models/refmods");
    std::fs::create_dir_all(&dir).ok();
    dir
}

/// List available RefMod names (relative paths under models/refmods).
fn list_mod_names() -> Vec<String> {
    let dir = refmods_dir();
    let mut names = Vec::new();
    if !dir.exists() {
        return names;
    }
    let Ok(entries) = fs::read_dir(&dir) else { return names; };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            // recurse one level for concept subfolders (identity/, appearance/, ...)
            if let Ok(sub) = fs::read_dir(&path) {
                for e in sub.flatten() {
                    let p = e.path();
                    if p.extension().map(|s| s == "safetensors").unwrap_or(false) {
                        if let (Some(parent), Some(stem)) = (p.parent(), p.file_stem()) {
                            let sub_name = parent.file_name()
                                .map(|n| n.to_string_lossy().to_string())
                                .unwrap_or_default();
                            names.push(format!("{}/{}", sub_name, stem.to_string_lossy()));
                        }
                    }
                }
            }
        } else if path.extension().map(|s| s == "safetensors").unwrap_or(false) {
            if let Some(stem) = path.file_stem() {
                names.push(stem.to_string_lossy().to_string());
            }
        }
    }
    names.sort();
    names
}

/// Resolve a mod name (possibly subfolder/name) to its .safetensors path.
fn resolve_mod_path(name: &str) -> Option<PathBuf> {
    let dir = refmods_dir();
    let direct = dir.join(format!("{}.safetensors", name));
    if direct.exists() {
        return Some(direct);
    }
    // subfolder/name → subfolder/name.safetensors
    let with_sub = dir.join(format!("{}.safetensors", name));
    if with_sub.exists() {
        return Some(with_sub);
    }
    None
}

/// Extract image file paths from an IMAGE input value.
fn resolve_image_paths(value: &Value, _node_id: &str) -> Vec<String> {
    let mut paths = Vec::new();
    if value.is_null() {
        return paths;
    }
    let try_path = |path: &str| -> Option<String> {
        let p = Path::new(path);
        if p.is_absolute() || path.starts_with("input/") || p.exists() {
            Some(path.to_string())
        } else {
            let under = format!("input/{}", path);
            if Path::new(&under).exists() {
                Some(under)
            } else {
                Some(path.to_string())
            }
        }
    };
    if let Some(obj) = value.as_object() {
        if let Some(path) = obj.get("path").and_then(|v| v.as_str()) {
            if !path.is_empty() {
                if let Some(p) = try_path(path) { paths.push(p); }
            }
        }
        if let Some(arr) = obj.get("images").and_then(|v| v.as_array()) {
            for item in arr {
                if let Some(path) = item.get("path").and_then(|v| v.as_str()) {
                    if !path.is_empty() {
                        if let Some(p) = try_path(path) { paths.push(p); }
                        continue;
                    }
                }
            }
        }
    }
    paths
}

fn str_input(inputs: &HashMap<String, Value>, key: &str, default: &str) -> String {
    inputs.get(key).and_then(|v| v.as_str()).unwrap_or(default).to_string()
}

fn int_input(inputs: &HashMap<String, Value>, key: &str, default: i64) -> i64 {
    inputs.get(key).and_then(|v| v.as_i64()).unwrap_or(default)
}

fn float_input(inputs: &HashMap<String, Value>, key: &str, default: f64) -> f64 {
    inputs.get(key).and_then(|v| v.as_f64()).unwrap_or(default)
}

// ── 1. Extract H3 RefMod ──────────────────────────────────────────────────

fn register_extract(registry: &mut NodeRegistry) {
    let class = NodeClassDef {
        class_type: "MiniMaxH3RefModExtract".to_string(),
        display_name: "Extract H3 RefMod (参考图→RefMod)".to_string(),
        category: "H3/ref2va".to_string(),
        input_types: NodeInputTypes {
            required: {
                let mut m = HashMap::new();
                m.insert("image".to_string(), InputTypeSpec {
                    type_name: "IMAGE".to_string(), extra: HashMap::new() });
                m.insert("vae_dir".to_string(), InputTypeSpec {
                    type_name: "STRING".to_string(), extra: HashMap::new() });
                m.insert("name".to_string(), InputTypeSpec {
                    type_name: "STRING".to_string(), extra: HashMap::new() });
                m
            },
            optional: {
                let mut m = HashMap::new();
                let mode_choices: Vec<Value> = ["training", "encode"].iter().map(|s| json!(s)).collect();
                m.insert("mode".to_string(), InputTypeSpec {
                    type_name: "COMBO".to_string(),
                    extra: { let mut e = HashMap::new(); e.insert("choices".to_string(), Value::Array(mode_choices)); e.insert("default".to_string(), json!("training")); e } });
                m.insert("resolution".to_string(), InputTypeSpec {
                    type_name: "INT".to_string(),
                    extra: { let mut e = HashMap::new(); e.insert("default".to_string(), json!(1024)); e.insert("min".to_string(), json!(256)); e.insert("max".to_string(), json!(2048)); e } });
                m.insert("pool".to_string(), InputTypeSpec {
                    type_name: "INT".to_string(),
                    extra: { let mut e = HashMap::new(); e.insert("default".to_string(), json!(16)); e.insert("min".to_string(), json!(2)); e.insert("max".to_string(), json!(64)); e } });
                m.insert("identity".to_string(), InputTypeSpec {
                    type_name: "INT".to_string(),
                    extra: { let mut e = HashMap::new(); e.insert("default".to_string(), json!(500)); e.insert("min".to_string(), json!(0)); e.insert("max".to_string(), json!(5000)); e } });
                let ct_choices: Vec<Value> = ["generic", "identity", "appearance", "clothing", "background", "environment", "style", "motion", "object"].iter().map(|s| json!(s)).collect();
                m.insert("concept_type".to_string(), InputTypeSpec {
                    type_name: "COMBO".to_string(),
                    extra: { let mut e = HashMap::new(); e.insert("choices".to_string(), Value::Array(ct_choices)); e.insert("default".to_string(), json!("generic")); e } });
                m.insert("description".to_string(), InputTypeSpec {
                    type_name: "STRING".to_string(), extra: HashMap::new() });
                m
            },
            hidden: HashMap::new(),
        },
        output_types: vec![IoType::Custom(REFMODS_IO.to_string()), IoType::String],
        output_names: vec!["REFMODS".to_string(), "PATH".to_string()],
        output_is_list: vec![false, false],
        is_output_node: false,
        has_intermediate_output: false,
        is_changed: None,
        not_idempotent: true,
        function_name: "extract".to_string(),
    };

    registry.register(class, Arc::new(move |ctx, node, node_id| {
        let image_val = ctx.resolve_input(node_id, "image").unwrap_or_else(|_| json!(null));
        let vae_dir = str_input(&node.inputs, "vae_dir", "");
        let name = str_input(&node.inputs, "name", "refmod");
        let mode = str_input(&node.inputs, "mode", "training");
        let resolution = int_input(&node.inputs, "resolution", 1024);
        let pool = int_input(&node.inputs, "pool", 16);
        let identity = int_input(&node.inputs, "identity", 500);
        let concept_type = str_input(&node.inputs, "concept_type", "generic");
        let description = str_input(&node.inputs, "description", "");
        let node_id_str = node_id.to_string();

        Box::pin(async move {
            let paths = resolve_image_paths(&image_val, &node_id_str);
            if paths.is_empty() {
                return Err(ExecutorError::NodeExecutionFailed {
                    node_id: node_id_str.clone(),
                    message: "Extract H3 RefMod: no image input".to_string(),
                });
            }
            if vae_dir.is_empty() {
                return Err(ExecutorError::NodeExecutionFailed {
                    node_id: node_id_str.clone(),
                    message: "Extract H3 RefMod: vae_dir is required".to_string(),
                });
            }
            let backend = ctx.backend();
            let saved = backend.refmod_extract(&paths, &vae_dir, &name, &mode,
                                               resolution, pool, identity,
                                               &concept_type, &description)
                .map_err(|e| ExecutorError::NodeExecutionFailed {
                    node_id: node_id_str.clone(),
                    message: format!("refmod extract failed: {}", e),
                })?;

            let bundle = json!({
                "mods": [{ "path": saved, "strength": 1.0, "copies": 1, "name": name }],
                "description": description,
            });
            Ok(vec![bundle, json!(saved)])
        })
    }));
}

// ── 2. Load H3 RefMods (8 slots) ──────────────────────────────────────────

fn register_loader(registry: &mut NodeRegistry) {
    let mod_names = list_mod_names();
    let mod_choices: Vec<Value> = std::iter::once(json!("none"))
        .chain(mod_names.iter().map(|s| json!(s)))
        .collect();

    let mut optional = HashMap::new();
    for i in 1..=8 {
        optional.insert(format!("mod_{}", i), InputTypeSpec {
            type_name: "COMBO".to_string(),
            extra: { let mut e = HashMap::new(); e.insert("choices".to_string(), Value::Array(mod_choices.clone())); e.insert("default".to_string(), json!("none")); e } });
        optional.insert(format!("strength_{}", i), InputTypeSpec {
            type_name: "FLOAT".to_string(),
            extra: { let mut e = HashMap::new(); e.insert("default".to_string(), json!(1.0)); e.insert("min".to_string(), json!(0.0)); e.insert("max".to_string(), json!(2.0)); e.insert("step".to_string(), json!(0.05)); e } });
        optional.insert(format!("copies_{}", i), InputTypeSpec {
            type_name: "INT".to_string(),
            extra: { let mut e = HashMap::new(); e.insert("default".to_string(), json!(1)); e.insert("min".to_string(), json!(1)); e.insert("max".to_string(), json!(10)); e } });
    }

    let class = NodeClassDef {
        class_type: "MiniMaxH3RefModsLoader".to_string(),
        display_name: "Load H3 RefMods (加载参考Mod)".to_string(),
        category: "H3/ref2va".to_string(),
        input_types: NodeInputTypes {
            required: HashMap::new(),
            optional,
            hidden: HashMap::new(),
        },
        output_types: vec![IoType::Custom(REFMODS_IO.to_string()), IoType::String],
        output_names: vec!["REFMODS".to_string(), "PROMPT_HINT".to_string()],
        output_is_list: vec![false, false],
        is_output_node: false,
        has_intermediate_output: false,
        is_changed: None,
        not_idempotent: false,
        function_name: "load".to_string(),
    };

    registry.register(class, Arc::new(move |_ctx, node, node_id| {
        let node_id_str = node_id.to_string();
        Box::pin(async move {
            let mut mods = Vec::new();
            let mut hints = Vec::new();
            for i in 1..=8 {
                let name = str_input(&node.inputs, &format!("mod_{}", i), "none");
                if name == "none" || name.is_empty() { continue; }
                let path = resolve_mod_path(&name).map(|p| p.to_string_lossy().to_string())
                    .unwrap_or_else(|| name.to_string());
                let strength = float_input(&node.inputs, &format!("strength_{}", i), 1.0);
                let copies = int_input(&node.inputs, &format!("copies_{}", i), 1);
                for _ in 0..copies {
                    mods.push(json!({
                        "path": path, "strength": strength, "copies": 1, "name": name
                    }));
                }
                hints.push(format!("{}×{}", name, copies));
            }
            let bundle = json!({ "mods": mods, "description": hints.join(", ") });
            Ok(vec![bundle, json!(hints.join(", "))])
        })
    }));
}

// ── 3. Load H3 RefMod Folder ──────────────────────────────────────────────

fn register_folder_loader(registry: &mut NodeRegistry) {
    let class = NodeClassDef {
        class_type: "MiniMaxH3RefModFolderLoader".to_string(),
        display_name: "Load H3 RefMod Folder (文件夹参考列表)".to_string(),
        category: "H3/ref2va".to_string(),
        input_types: NodeInputTypes {
            required: {
                let mut m = HashMap::new();
                m.insert("folder".to_string(), InputTypeSpec {
                    type_name: "STRING".to_string(), extra: HashMap::new() });
                m
            },
            optional: HashMap::new(),
            hidden: HashMap::new(),
        },
        output_types: vec![IoType::Image, IoType::Int],
        output_names: vec!["IMAGES".to_string(), "COUNT".to_string()],
        output_is_list: vec![true, false],
        is_output_node: false,
        has_intermediate_output: false,
        is_changed: None,
        not_idempotent: false,
        function_name: "load_folder".to_string(),
    };

    registry.register(class, Arc::new(move |_ctx, node, node_id| {
        let folder = str_input(&node.inputs, "folder", "").to_string();
        let node_id_str = node_id.to_string();
        Box::pin(async move {
            if folder.is_empty() {
                return Err(ExecutorError::NodeExecutionFailed {
                    node_id: node_id_str.clone(),
                    message: "folder is required".to_string(),
                });
            }
            let dir = Path::new(&folder);
            if !dir.is_dir() {
                return Err(ExecutorError::NodeExecutionFailed {
                    node_id: node_id_str.clone(),
                    message: format!("folder not found: {}", folder),
                });
            }
            let exts = ["png", "jpg", "jpeg", "webp", "bmp", "gif", "mp4", "mov", "webm", "avi"];
            let mut files: Vec<PathBuf> = Vec::new();
            if let Ok(entries) = fs::read_dir(dir) {
                for e in entries.flatten() {
                    let p = e.path();
                    if p.is_file() {
                        if let Some(ext) = p.extension().and_then(|s| s.to_str()) {
                            if exts.contains(&ext.to_lowercase().as_str()) {
                                files.push(p);
                            }
                        }
                    }
                }
            }
            files.sort();
            // For now return images only (videos need the VAE pipeline);
            // the folder is ordered and feeds the Extract node's image input.
            let img_exts = ["png", "jpg", "jpeg", "webp", "bmp"];
            let images: Vec<Value> = files.iter()
                .filter(|p| p.extension().and_then(|s| s.to_str())
                    .map(|e| img_exts.contains(&e.to_lowercase().as_str())).unwrap_or(false))
                .map(|p| json!({ "type": "image", "path": p.to_string_lossy().to_string() }))
                .collect();
            let count = images.len() as i64;
            Ok(vec![json!({ "type": "image", "images": images }), json!(count)])
        })
    }));
}

// ── 4. Apply H3 RefMod ────────────────────────────────────────────────────

fn register_apply(registry: &mut NodeRegistry) {
    let class = NodeClassDef {
        class_type: "MiniMaxH3RefModApply".to_string(),
        display_name: "Apply H3 RefMod (注入参考到H3)".to_string(),
        category: "H3/ref2va".to_string(),
        input_types: NodeInputTypes {
            required: {
                let mut m = HashMap::new();
                m.insert("mods".to_string(), InputTypeSpec {
                    type_name: REFMODS_IO.to_string(), extra: HashMap::new() });
                m.insert("vae_dir".to_string(), InputTypeSpec {
                    type_name: "STRING".to_string(), extra: HashMap::new() });
                m
            },
            optional: {
                let mut m = HashMap::new();
                m.insert("cond".to_string(), InputTypeSpec {
                    type_name: "CONDITIONING".to_string(), extra: HashMap::new() });
                m.insert("strength".to_string(), InputTypeSpec {
                    type_name: "FLOAT".to_string(),
                    extra: { let mut e = HashMap::new(); e.insert("default".to_string(), json!(1.0)); e.insert("min".to_string(), json!(0.0)); e.insert("max".to_string(), json!(2.0)); e.insert("step".to_string(), json!(0.05)); e } });
                m
            },
            hidden: HashMap::new(),
        },
        output_types: vec![IoType::Image, IoType::Conditioning],
        output_names: vec!["IMAGE".to_string(), "CONDITIONING".to_string()],
        output_is_list: vec![false, false],
        is_output_node: false,
        has_intermediate_output: false,
        is_changed: None,
        not_idempotent: true,
        function_name: "apply".to_string(),
    };

    registry.register(class, Arc::new(move |ctx, node, node_id| {
        let cond_val = ctx.resolve_input(node_id, "cond").unwrap_or_else(|_| json!(null));
        let mods_val = ctx.resolve_input(node_id, "mods").unwrap_or_else(|_| json!(null));
        let vae_dir = str_input(&node.inputs, "vae_dir", "").to_string();
        let global_strength = float_input(&node.inputs, "strength", 1.0);
        let node_id_str = node_id.to_string();

        Box::pin(async move {
            let mods_arr = mods_val.get("mods").and_then(|v| v.as_array())
                .cloned().unwrap_or_default();
            if mods_arr.is_empty() {
                // No mods: pass through the IMAGE as empty and cond unchanged.
                let empty_img = json!({ "type": "image", "images": [] });
                return Ok(vec![empty_img, cond_val]);
            }
            if vae_dir.is_empty() {
                return Err(ExecutorError::NodeExecutionFailed {
                    node_id: node_id_str.clone(),
                    message: "Apply H3 RefMod: vae_dir is required".to_string(),
                });
            }

            let backend = ctx.backend();
            let mut ref_image_vals: Vec<Value> = Vec::new();
            for m in &mods_arr {
                let path = m.get("path").and_then(|v| v.as_str()).unwrap_or("");
                let strength = m.get("strength").and_then(|v| v.as_f64()).unwrap_or(1.0)
                    * global_strength;
                if path.is_empty() || strength <= 0.0 { continue; }
                match backend.refmod_decode(path, &vae_dir) {
                    Ok(img) => {
                        // Save decoded image to a temp PNG so the downstream
                        // MiniMaxH3Pipeline (which resolves IMAGE from paths)
                        // can consume it as a ref2va reference.
                        let tmp = std::env::temp_dir().join("comfyui-rust");
                        std::fs::create_dir_all(&tmp).ok();
                        let fname = format!("refmod_decoded_{}_{}.png",
                            std::process::id(),
                            std::time::SystemTime::now()
                                .duration_since(std::time::UNIX_EPOCH)
                                .map(|d| d.as_nanos()).unwrap_or(0));
                        let out_path = tmp.join(&fname);
                        if let Ok(png) = img.to_png_bytes() {
                            std::fs::write(&out_path, png).ok();
                            let pstr = out_path.to_string_lossy().to_string();
                            ref_image_vals.push(json!({ "type": "image", "path": pstr }));
                        }
                    }
                    Err(e) => {
                        tracing::warn!("Apply H3 RefMod: decode failed for {}: {}", path, e);
                    }
                }
            }

            // IMAGE output: {images: [{path}, ...]} for the pipeline's
            // ref2va reference input.
            let image_out = json!({ "type": "image", "images": ref_image_vals });

            // CONDITIONING output: inject decoded reference images into the
            // H3 params cond for any node that consumes it directly.
            let mut cond = cond_val.clone();
            if let Some(obj) = cond.as_object_mut() {
                let existing = obj.get("reference_images")
                    .and_then(|v| v.as_array()).cloned().unwrap_or_default();
                let mut merged = existing;
                merged.extend(ref_image_vals.clone());
                obj.insert("reference_images".to_string(), Value::Array(merged));
                if obj.get("mode").and_then(|v| v.as_str()) == Some("t2va") {
                    obj.insert("mode".to_string(), json!("i2va"));
                }
            }
            Ok(vec![image_out, cond])
        })
    }));
}

// ── 5. Load H3 RefMod Axis (A/B signed sliders) ───────────────────────────

fn register_axis(registry: &mut NodeRegistry) {
    let mod_names = list_mod_names();
    let mod_choices: Vec<Value> = std::iter::once(json!("none"))
        .chain(mod_names.iter().map(|s| json!(s)))
        .collect();

    let mut optional = HashMap::new();
    for i in 1..=8 {
        optional.insert(format!("mod_a_{}", i), InputTypeSpec {
            type_name: "COMBO".to_string(),
            extra: { let mut e = HashMap::new(); e.insert("choices".to_string(), Value::Array(mod_choices.clone())); e.insert("default".to_string(), json!("none")); e } });
        optional.insert(format!("mod_b_{}", i), InputTypeSpec {
            type_name: "COMBO".to_string(),
            extra: { let mut e = HashMap::new(); e.insert("choices".to_string(), Value::Array(mod_choices.clone())); e.insert("default".to_string(), json!("none")); e } });
        optional.insert(format!("value_{}", i), InputTypeSpec {
            type_name: "FLOAT".to_string(),
            extra: { let mut e = HashMap::new(); e.insert("default".to_string(), json!(0.0)); e.insert("min".to_string(), json!(-1.0)); e.insert("max".to_string(), json!(1.0)); e.insert("step".to_string(), json!(0.05)); e } });
    }

    let class = NodeClassDef {
        class_type: "MiniMaxH3RefModsAxis".to_string(),
        display_name: "Load H3 RefMod Axis (A/B参考轴)".to_string(),
        category: "H3/ref2va".to_string(),
        input_types: NodeInputTypes {
            required: HashMap::new(),
            optional,
            hidden: HashMap::new(),
        },
        output_types: vec![IoType::Custom(REFMODS_IO.to_string())],
        output_names: vec!["REFMODS".to_string()],
        output_is_list: vec![false],
        is_output_node: false,
        has_intermediate_output: false,
        is_changed: None,
        not_idempotent: false,
        function_name: "axis".to_string(),
    };

    registry.register(class, Arc::new(move |_ctx, node, node_id| {
        let node_id_str = node_id.to_string();
        Box::pin(async move {
            let mut mods = Vec::new();
            for i in 1..=8 {
                let a = str_input(&node.inputs, &format!("mod_a_{}", i), "none");
                let b = str_input(&node.inputs, &format!("mod_b_{}", i), "none");
                let value = float_input(&node.inputs, &format!("value_{}", i), 0.0);
                let (chosen, sign) = if value < 0.0 && a != "none" {
                    (a, -value)
                } else if value > 0.0 && b != "none" {
                    (b, value)
                } else {
                    continue;
                };
                let path = resolve_mod_path(&chosen).map(|p| p.to_string_lossy().to_string())
                    .unwrap_or_else(|| chosen.to_string());
                mods.push(json!({
                    "path": path, "strength": sign, "copies": 1, "name": chosen
                }));
            }
            let bundle = json!({ "mods": mods });
            Ok(vec![bundle])
        })
    }));
}

pub fn register_refmod_nodes(registry: &mut NodeRegistry) {
    register_extract(registry);
    register_loader(registry);
    register_folder_loader(registry);
    register_apply(registry);
    register_axis(registry);
}

//! Boogu-Image 控制节点：
//! - `BooguImagePipeline`：Boogu-Image 文生图与图像编辑（Base / Edit / Turbo /
//!   Edit-Turbo）。模型根目录下自包含的 diffusers 仓库（如
//!   `Boogu-Image-0.1-Edit-Turbo/`，内含 transformer/vae 分片）由推理层
//!   `FallbackBackend::rewrite_pipeline_for_sdcpp` 自动改写为 stable-diffusion.cpp
//!   的原生组件路径（diffusion 分片索引 + 仓库自带的 FLUX VAE + text_encoders
//!   下的 Qwen3-VL 权重，编辑场景再挂 llm_vision），无需 Python。

use crate::error::ExecutorError;
use crate::registry::NodeRegistry;
use comfy_core::{InputTypeSpec, IoType, NodeClassDef, NodeInputTypes};
use comfy_inference::image::SdImage;
use comfy_inference::params::{ImageGenParams, ModelConfig};
use comfy_inference::types::SampleMethod;
use serde_json::{json, Value};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

fn get_models_dir() -> String {
    std::env::var("COMFY_MODELS_DIR").unwrap_or_else(|_| "models".to_string())
}

/// Read the diffusers pipeline class from `<dir>/model_index.json`
/// (all Boogu variants report `BooguImagePipeline`).
fn pipeline_class(dir: &Path) -> Option<String> {
    let text = fs::read_to_string(dir.join("model_index.json")).ok()?;
    let v: Value = serde_json::from_str(&text).ok()?;
    v.get("_class_name")
        .and_then(|x| x.as_str())
        .map(str::to_string)
}

/// A directory is a usable Boogu-Image pipeline when its name mentions boogu,
/// it ships the diffusers component layout, and its pipeline class belongs to
/// the Boogu family.
fn is_boogu_dir(dir: &Path) -> bool {
    let name = dir
        .file_name()
        .map(|n| n.to_string_lossy().to_lowercase())
        .unwrap_or_default();
    if !dir.is_dir() || !name.contains("boogu") {
        return false;
    }
    if !dir.join("transformer").is_dir() || !dir.join("vae").is_dir() {
        return false;
    }
    match pipeline_class(dir) {
        Some(class) => class.to_lowercase().contains("boogu"),
        // No/unknown pipeline metadata: accept the layout-based match only
        // when a transformer shard index is actually present.
        None => fs::read_dir(dir.join("transformer"))
            .ok()
            .map(|entries| {
                entries.flatten().any(|e| {
                    e.file_name()
                        .to_string_lossy()
                        .ends_with(".safetensors.index.json")
                })
            })
            .unwrap_or(false),
    }
}

/// Scan the model roots (root + diffusion_models/) for Boogu-Image pipelines.
/// Returns `(label, path)` pairs sorted by label.
fn scan_boogu_dirs() -> Vec<(String, PathBuf)> {
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
            if is_boogu_dir(&path) {
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

fn resolve_boogu_dir(label: &str) -> Option<PathBuf> {
    scan_boogu_dirs()
        .into_iter()
        .find(|(name, _)| name == label)
        .map(|(_, path)| path)
}

/// Whether the pipeline variant performs image editing (Edit / Edit-Turbo).
/// All variants share the `BooguImagePipeline` class, so the variant is taken
/// from the directory name.
fn dir_is_edit(dir: &Path) -> bool {
    dir.file_name()
        .map(|n| n.to_string_lossy().to_lowercase().contains("edit"))
        .unwrap_or(false)
}

/// Decode an IMAGE input value (LoadImage `{path}` / `{images:[…]}` envelope
/// or an inline serialized `SdImage`) into decoded reference images.
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
    match serde_json::from_value::<SdImage>(value.clone()) {
        Ok(img) => Ok(vec![img]),
        Err(_) => Ok(Vec::new()),
    }
}

/// Round a dimension down to the required 16-pixel multiple (Boogu uses the
/// FLUX VAE + 2x patchify: dimensions must be divisible by 16), clamped to
/// `[256, 2048]`.
fn quantize_dim(v: i64) -> i32 {
    let q = ((v.max(256).min(2048)) / 16 * 16) as i32;
    q.max(256)
}

fn register_boogu_image_pipeline(registry: &mut NodeRegistry) {
    // The closure is Fn and must rescan on every invocation (captured Vec
    // would be moved); see bernini/minimax/qwen nodes for the same pattern.
    let model_choices: Vec<String> = scan_boogu_dirs().into_iter().map(|(n, _)| n).collect();
    let default_model = model_choices.first().cloned().unwrap_or_default();

    let class_def = NodeClassDef {
        class_type: "BooguImagePipeline".to_string(),
        display_name: "Boogu-Image 文生图/图像编辑 (Base/Edit/Turbo)".to_string(),
        category: "image/boogu".to_string(),
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
                m.insert("boogu_model".to_string(), InputTypeSpec {
                    type_name: "COMBO".to_string(),
                    extra: {
                        let mut e = HashMap::new();
                        e.insert(
                            "choices".to_string(),
                            Value::Array(model_choices.iter().map(|s| json!(s)).collect()),
                        );
                        e
                    },
                });
                m
            },
            optional: {
                let mut m = HashMap::new();
                // Reference image for Edit / Edit-Turbo (-r). Boogu-Edit
                // currently supports exactly one reference image.
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
                // Distilled Turbo / Edit-Turbo need 3~4 steps; Base / Edit
                // use 25~50.
                m.insert("steps".to_string(), InputTypeSpec {
                    type_name: "INT".to_string(),
                    extra: {
                        let mut e = HashMap::new();
                        e.insert("min".to_string(), json!(1));
                        e.insert("max".to_string(), json!(100));
                        e.insert("step".to_string(), json!(1));
                        e.insert("default".to_string(), json!(4));
                        e
                    },
                });
                // Turbo variants: cfg 1.0; Base ~4.0, Edit ~5.0.
                m.insert("cfg".to_string(), InputTypeSpec {
                    type_name: "FLOAT".to_string(),
                    extra: {
                        let mut e = HashMap::new();
                        e.insert("min".to_string(), json!(0.0));
                        e.insert("max".to_string(), json!(20.0));
                        e.insert("step".to_string(), json!(0.1));
                        e.insert("default".to_string(), json!(1.0));
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
                        e.insert("default".to_string(), json!(1024));
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
                        e.insert("default".to_string(), json!(1024));
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
        not_idempotent: true,
        function_name: "run".to_string(),
    };

    registry.register(class_def, Arc::new(move |ctx, node, node_id| {
        let image_in = ctx.resolve_input(node_id, "image").unwrap_or_else(|_| json!(null));
        let model_label = node
            .inputs
            .get("boogu_model")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .unwrap_or_else(|| default_model.clone());
        let prompt = node
            .inputs
            .get("prompt")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let negative_prompt = node
            .inputs
            .get("negative_prompt")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let seed = node.inputs.get("seed").and_then(|v| v.as_i64()).unwrap_or(42);
        let steps = node.inputs.get("steps").and_then(|v| v.as_i64()).unwrap_or(4) as i32;
        let cfg = node
            .inputs
            .get("cfg")
            .and_then(|v| v.as_f64())
            .unwrap_or(1.0) as f32;
        let width = quantize_dim(
            node.inputs.get("width").and_then(|v| v.as_i64()).unwrap_or(1024),
        );
        let height = quantize_dim(
            node.inputs.get("height").and_then(|v| v.as_i64()).unwrap_or(1024),
        );
        let node_id_str = node_id.to_string();

        Box::pin(async move {
            if prompt.trim().is_empty() {
                return Err(ExecutorError::NodeExecutionFailed {
                    node_id: node_id_str.clone(),
                    message: "BooguImagePipeline requires a prompt".to_string(),
                });
            }

            let model_dir = resolve_boogu_dir(&model_label).ok_or_else(|| {
                ExecutorError::NodeExecutionFailed {
                    node_id: node_id_str.clone(),
                    message: format!(
                        "Boogu-Image model directory '{}' not found under COMFY_MODELS_DIR \
                         (need a self-contained Boogu-Image-* diffusers directory with \
                         transformer/ and vae/)",
                        model_label
                    ),
                }
            })?;

            let mut ref_images = resolve_input_images(&image_in, &node_id_str)?;
            let is_edit = dir_is_edit(&model_dir);
            if !ref_images.is_empty() && !is_edit {
                tracing::warn!(
                    "BooguImagePipeline: reference image provided but variant '{}' is not an \
                     Edit model; the backend will ignore it",
                    model_label
                );
            }
            // Boogu-Edit currently accepts exactly one reference image.
            if ref_images.len() > 1 {
                tracing::warn!(
                    "BooguImagePipeline: {} reference images provided but Boogu-Edit supports \
                     only one; using the first",
                    ref_images.len()
                );
                ref_images.truncate(1);
            }

            let mut params = ImageGenParams::new(prompt)
                .with_seed(seed)
                .with_sample_steps(steps)
                .with_cfg_scale(cfg)
                .with_sample_method(SampleMethod::Euler)
                .with_dimensions(width, height)
                .with_model_config(
                    ModelConfig::new().with_model(model_dir.to_string_lossy().to_string()),
                );
            if !negative_prompt.is_empty() {
                params = params.with_negative_prompt(negative_prompt);
            }
            params.ref_images = ref_images;

            let backend = ctx.backend();
            let images = backend
                .generate_image(params)
                .map_err(ExecutorError::Inference)?;

            let image_data: Vec<Value> = images
                .iter()
                .map(|img| {
                    serde_json::to_value(img).unwrap_or_else(|_| {
                        json!({
                            "type": "image",
                            "width": img.width,
                            "height": img.height,
                            "channel": img.channel,
                        })
                    })
                })
                .collect();
            Ok(vec![json!({
                "type": "image",
                "images": image_data,
            })])
        })
    }));
}

pub fn register_boogu_nodes(registry: &mut NodeRegistry) {
    register_boogu_image_pipeline(registry);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write_index(dir: &Path, class: &str) {
        fs::write(
            dir.join("model_index.json"),
            format!("{{\"_class_name\": \"{}\"}}", class),
        )
        .unwrap();
    }

    fn mkdir_pipeline(root: &Path, name: &str, class: Option<&str>, with_index: bool) {
        let dir = root.join(name);
        fs::create_dir_all(dir.join("transformer")).unwrap();
        fs::create_dir_all(dir.join("vae")).unwrap();
        match class {
            Some(c) => write_index(&dir, c),
            None if with_index => fs::write(
                dir.join("transformer")
                    .join("diffusion_pytorch_model.safetensors.index.json"),
                b"{}",
            )
            .unwrap(),
            None => {}
        }
    }

    #[test]
    fn scans_only_boogu_pipeline_dirs() {
        let _env = crate::TEST_ENV_LOCK.lock().unwrap();
        let root = std::env::temp_dir()
            .join(format!("comfy_boogu_scan_test_{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();

        mkdir_pipeline(&root, "Boogu-Image-0.1-Base", Some("BooguImagePipeline"), false);
        mkdir_pipeline(
            &root,
            "Boogu-Image-0.1-Edit-Turbo",
            Some("BooguImagePipeline"),
            false,
        );
        mkdir_pipeline(&root, "boogu_sharded", None, true);
        // Rejected: unrelated diffusers repo.
        mkdir_pipeline(&root, "Wan2.1", Some("WanPipeline"), false);
        // Rejected: boogu name + components but unrelated pipeline class.
        mkdir_pipeline(&root, "Boogu-Weird", Some("SomeOtherPipeline"), false);
        // Rejected: name matches but no components.
        let bare = root.join("Boogu-Bare");
        fs::create_dir_all(&bare).unwrap();
        write_index(&bare, "BooguImagePipeline");

        unsafe { std::env::set_var("COMFY_MODELS_DIR", &root); }
        let found: Vec<String> = scan_boogu_dirs().into_iter().map(|(n, _)| n).collect();
        assert_eq!(
            found,
            vec![
                "Boogu-Image-0.1-Base".to_string(),
                "Boogu-Image-0.1-Edit-Turbo".to_string(),
                "boogu_sharded".to_string(),
            ]
        );

        let edit = resolve_boogu_dir("Boogu-Image-0.1-Edit-Turbo").unwrap();
        assert!(dir_is_edit(&edit));
        let base = resolve_boogu_dir("Boogu-Image-0.1-Base").unwrap();
        assert!(!dir_is_edit(&base));
        assert!(resolve_boogu_dir("Boogu-Weird").is_none());

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn quantizes_dimensions_to_multiple_of_16() {
        assert_eq!(quantize_dim(1024), 1024);
        assert_eq!(quantize_dim(1000), 992);
        assert_eq!(quantize_dim(100), 256);
        assert_eq!(quantize_dim(3000), 2048);
    }

    #[test]
    fn boogu_node_is_registered_in_builtin_registry() {
        let _env = crate::TEST_ENV_LOCK.lock().unwrap();
        let real_models = Path::new("/home/acproject/usb/comfyui/models");
        if real_models.exists() {
            unsafe { std::env::set_var("COMFY_MODELS_DIR", real_models); }
        }
        let mut registry = NodeRegistry::new();
        crate::builtin_nodes::register_builtin_nodes(&mut registry);

        let def = registry
            .get_class_def("BooguImagePipeline")
            .expect("BooguImagePipeline must be registered");
        assert_eq!(def.category, "image/boogu");
        assert_eq!(def.output_types, vec![IoType::Image]);
        assert_eq!(def.output_names, vec!["IMAGE".to_string()]);
        assert!(def.input_types.required.contains_key("prompt"));
        assert!(def.input_types.required.contains_key("boogu_model"));
        assert!(def.input_types.optional.contains_key("image"));
        assert!(def.input_types.optional.contains_key("cfg"));
        // Turbo defaults.
        let steps_default = def
            .input_types
            .optional
            .get("steps")
            .and_then(|s| s.extra.get("default"))
            .and_then(|v| v.as_i64());
        assert_eq!(steps_default, Some(4));

        if real_models.exists() {
            let choices = def
                .input_types
                .required
                .get("boogu_model")
                .and_then(|s| s.extra.get("choices"))
                .and_then(|v| v.as_array())
                .expect("model choices");
            let labels: Vec<&str> = choices.iter().filter_map(|c| c.as_str()).collect();
            assert!(
                labels.iter().any(|l| l == &"Boogu-Image-0.1-Edit-Turbo"),
                "Boogu-Image-0.1-Edit-Turbo dir should appear in combo: {labels:?}"
            );
        }
    }
}

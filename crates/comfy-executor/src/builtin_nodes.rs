use crate::error::ExecutorError;
use crate::registry::NodeRegistry;
use comfy_core::{IoType, NodeClassDef, NodeInputTypes, InputTypeSpec};
use comfy_inference::{ImageGenParams, ModelConfig, SampleMethod, Scheduler};
#[cfg(feature = "local-ffi")]
use comfy_inference::{ConvertParams, convert_model, SdType};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::Arc;

#[derive(Debug, Clone, Copy, PartialEq)]
enum ModelType {
    SD3,
    Flux,
    SDXL,
    SD15,
    Wan,
    Unknown,
}

fn detect_model_type(checkpoint_name: &str) -> ModelType {
    let lower = checkpoint_name.to_lowercase();
    if lower.contains("sd3") || lower.contains("sd3.5") {
        ModelType::SD3
    } else if lower.contains("flux") {
        ModelType::Flux
    } else if lower.contains("sdxl") {
        ModelType::SDXL
    } else if lower.contains("wan") {
        ModelType::Wan
    } else if lower.contains("v1") || lower.contains("sd1") || lower.contains("stable-diffusion-1") {
        ModelType::SD15
    } else {
        ModelType::Unknown
    }
}

fn get_models_base_dir() -> std::path::PathBuf {
    let base = std::env::var("COMFY_MODELS_DIR").unwrap_or_else(|_| "models".to_string());
    let base_path = std::path::Path::new(&base);
    if base_path.is_relative() {
        std::env::current_dir().unwrap_or_default().join(base_path)
    } else {
        base_path.to_path_buf()
    }
}

fn find_file_in_dir(dir: &std::path::Path, prefixes: &[&str]) -> Option<String> {
    if !dir.exists() {
        return None;
    }
    if let Ok(entries) = std::fs::read_dir(dir) {
        let mut candidates: Vec<String> = entries
            .filter_map(|e| e.ok())
            .filter_map(|e| {
                let name = e.file_name().to_string_lossy().to_string();
                let lower = name.to_lowercase();
                if lower.ends_with(".safetensors") || lower.ends_with(".gguf") {
                    prefixes.iter().any(|p| lower.starts_with(p)).then(|| name)
                } else {
                    None
                }
            })
            .collect();
        candidates.sort();
        return candidates.first().map(|name| dir.join(name).to_string_lossy().to_string());
    }
    None
}

fn auto_detect_text_encoders(model_type: ModelType) -> (Option<String>, Option<String>, Option<String>) {
    let base = get_models_base_dir();
    let te_dir = base.join("text_encoders");

    let (need_clip_l, need_clip_g, need_t5xxl) = match model_type {
        ModelType::SD3 => (true, true, true),
        ModelType::Flux => (true, false, true),
        ModelType::SDXL => (true, true, false),
        ModelType::SD15 => (true, false, false),
        ModelType::Wan => (false, false, true),
        ModelType::Unknown => (true, true, true),
    };

    let clip_l_path = if need_clip_l {
        find_file_in_dir(&te_dir, &["clip_l"])
    } else {
        None
    };
    let clip_g_path = if need_clip_g {
        find_file_in_dir(&te_dir, &["clip_g"])
    } else {
        None
    };
    let t5xxl_path = if need_t5xxl {
        find_file_in_dir(&te_dir, &["t5xxl"])
    } else {
        None
    };

    (clip_l_path, clip_g_path, t5xxl_path)
}

fn auto_detect_vae(model_type: ModelType) -> Option<String> {
    let base = get_models_base_dir();
    let vae_dir = base.join("vae");

    let prefixes: &[&str] = match model_type {
        ModelType::SD3 | ModelType::Flux => &["sd3_vae", "flux_vae", "ae"],
        ModelType::SDXL | ModelType::SD15 => &["sdxl_vae", "vae"],
        ModelType::Wan => &["wan_vae"],
        ModelType::Unknown => &["sd3_vae", "flux_vae", "sdxl_vae", "vae", "ae"],
    };

    find_file_in_dir(&vae_dir, prefixes)
}

pub fn register_builtin_nodes(registry: &mut NodeRegistry) {
    register_checkpoint_loader(registry);
    register_flux_loader(registry);
    register_sd3_loader(registry);
    register_wan_loader(registry);
    register_diffusion_model_loader(registry);
    register_vae_loader(registry);
    register_lora_loader(registry);
    register_clip_loader(registry);
    register_dual_clip_loader(registry);
    register_clip_text_encode(registry);
    register_ksampler(registry);
    register_save_image(registry);
    register_empty_latent_image(registry);
    register_vae_decode(registry);
    register_vae_encode(registry);
    register_load_image(registry);
    register_upscale_image(registry);
    register_clip_vision_encode(registry);
    register_control_net_apply(registry);
    register_convert_model(registry);
    register_wan_video_sampler(registry);
    register_ltx_loader(registry);
    register_ltx_video_sampler(registry);
    register_llm_loader(registry);
    register_llm_text_gen(registry);
    register_llm_text_gen_remote(registry);
    register_save_video(registry);
    register_load_video(registry);
    register_load_audio(registry);
    register_save_audio(registry);
    register_audio_to_llm(registry);

    #[cfg(feature = "controlnet")]
    crate::controlnet::register_controlnet_nodes(registry);

    crate::mask::register_mask_nodes(registry);
}

fn resolve_model_path(model_type: &str, filename: &str) -> String {
    let base = std::env::var("COMFY_MODELS_DIR").unwrap_or_else(|_| "models".to_string());
    let sub_dir = match model_type {
        "checkpoints" => "checkpoints",
        "clip" | "text_encoders" => "text_encoders",
        "vae" => "vae",
        "loras" => "loras",
        "controlnet" => "controlnet",
        "upscale_models" => "upscale_models",
        "embeddings" => "embeddings",
        "diffusion_models" => "diffusion_models",
        "clip_vision" => "clip_vision",
        "style_models" => "style_models",
        "diffusers" => "diffusers",
        "vae_approx" => "vae_approx",
        "gligen" => "gligen",
        "latent_upscale_models" => "latent_upscale_models",
        "hypernetworks" => "hypernetworks",
        "photomarker" => "photomarker",
        "classifiers" => "classifiers",
        "model_patches" => "model_patches",
        "audio_encoders" => "audio_encoders",
        _ => model_type,
    };
    std::path::Path::new(&base)
        .join(sub_dir)
        .join(filename)
        .to_string_lossy()
        .to_string()
}

fn register_checkpoint_loader(registry: &mut NodeRegistry) {
    let class_def = NodeClassDef {
        class_type: "CheckpointLoaderSimple".to_string(),
        display_name: "Load Checkpoint".to_string(),
        category: "loaders".to_string(),
        input_types: NodeInputTypes {
            required: {
                let mut m = HashMap::new();
                m.insert("ckpt_name".to_string(), InputTypeSpec {
                    type_name: "COMBO".to_string(),
                    extra: HashMap::new(),
                });
                m
            },
            optional: HashMap::new(),
            hidden: HashMap::new(),
        },
        output_types: vec![IoType::Model, IoType::Clip, IoType::Vae],
        output_names: vec!["MODEL".to_string(), "CLIP".to_string(), "VAE".to_string()],
        output_is_list: vec![false, false, false],
        is_output_node: false,
        has_intermediate_output: false,
        is_changed: None,
        not_idempotent: false,
        function_name: "load".to_string(),
    };

    registry.register(class_def, Arc::new(|_ctx, node, _node_id| {
        let ckpt_name = node.inputs.get("ckpt_name")
            .and_then(|v| v.as_str())
            .unwrap_or("model.safetensors");

        let model_path = resolve_model_path("checkpoints", ckpt_name);
        let model_type = detect_model_type(ckpt_name);
        let model_type_str = format!("{:?}", model_type).to_lowercase();

        Box::pin(async move {
            let is_gguf = model_path.to_lowercase().ends_with(".gguf");

            let model_config = if is_gguf {
                json!({
                    "diffusion_model_path": model_path,
                    "model_type": model_type_str,
                })
            } else {
                json!({
                    "model_path": model_path,
                    "model_type": model_type_str,
                })
            };

            tracing::info!("CheckpointLoader: model_config = {}", serde_json::to_string_pretty(&model_config).unwrap_or_default());

            let clip_config = json!({
                "type": "clip",
                "source_model": model_path,
                "model_type": model_type_str,
            });

            let vae_config = json!({
                "type": "vae",
                "source_model": model_path,
                "model_type": model_type_str,
            });

            Ok(vec![model_config, clip_config, vae_config])
        })
    }));
}

fn register_flux_loader(registry: &mut NodeRegistry) {
    let class_def = NodeClassDef {
        class_type: "UNETLoader".to_string(),
        display_name: "Load Diffusion Model".to_string(),
        category: "loaders".to_string(),
        input_types: NodeInputTypes {
            required: {
                let mut m = HashMap::new();
                m.insert("unet_name".to_string(), InputTypeSpec {
                    type_name: "COMBO".to_string(),
                    extra: HashMap::new(),
                });
                m.insert("weight_dtype".to_string(), InputTypeSpec {
                    type_name: "COMBO".to_string(),
                    extra: HashMap::new(),
                });
                m
            },
            optional: HashMap::new(),
            hidden: HashMap::new(),
        },
        output_types: vec![IoType::Model],
        output_names: vec!["MODEL".to_string()],
        output_is_list: vec![false],
        is_output_node: false,
        has_intermediate_output: false,
        is_changed: None,
        not_idempotent: false,
        function_name: "load_unet".to_string(),
    };

    registry.register(class_def, Arc::new(|_ctx, node, _node_id| {
        let unet_name = node.inputs.get("unet_name")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        let diffusion_model_path = resolve_model_path("diffusion_models", unet_name);

        Box::pin(async move {
            Ok(vec![json!({
                "diffusion_model_path": diffusion_model_path,
            })])
        })
    }));
}

fn register_sd3_loader(registry: &mut NodeRegistry) {
    let class_def = NodeClassDef {
        class_type: "SD3Loader".to_string(),
        display_name: "Load SD3 Model".to_string(),
        category: "loaders".to_string(),
        input_types: NodeInputTypes {
            required: {
                let mut m = HashMap::new();
                m.insert("model_name".to_string(), InputTypeSpec {
                    type_name: "COMBO".to_string(),
                    extra: HashMap::new(),
                });
                m
            },
            optional: HashMap::new(),
            hidden: HashMap::new(),
        },
        output_types: vec![IoType::Model, IoType::Clip, IoType::Vae],
        output_names: vec!["MODEL".to_string(), "CLIP".to_string(), "VAE".to_string()],
        output_is_list: vec![false, false, false],
        is_output_node: false,
        has_intermediate_output: false,
        is_changed: None,
        not_idempotent: false,
        function_name: "load_sd3".to_string(),
    };

    registry.register(class_def, Arc::new(|_ctx, node, _node_id| {
        let model_name = node.inputs.get("model_name")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        let model_path = resolve_model_path("checkpoints", model_name);

        Box::pin(async move {
            let model_config = json!({
                "model_path": model_path,
            });
            let clip_config = json!({
                "type": "clip",
                "source_model": model_path,
            });
            let vae_config = json!({
                "type": "vae",
                "source_model": model_path,
            });
            Ok(vec![model_config, clip_config, vae_config])
        })
    }));
}

fn register_wan_loader(registry: &mut NodeRegistry) {
    let class_def = NodeClassDef {
        class_type: "WanLoader".to_string(),
        display_name: "Load Wan Model".to_string(),
        category: "loaders".to_string(),
        input_types: NodeInputTypes {
            required: {
                let mut m = HashMap::new();
                m.insert("model_name".to_string(), InputTypeSpec {
                    type_name: "COMBO".to_string(),
                    extra: HashMap::new(),
                });
                m
            },
            optional: HashMap::new(),
            hidden: HashMap::new(),
        },
        output_types: vec![IoType::Model, IoType::Clip, IoType::Vae],
        output_names: vec!["MODEL".to_string(), "CLIP".to_string(), "VAE".to_string()],
        output_is_list: vec![false, false, false],
        is_output_node: false,
        has_intermediate_output: false,
        is_changed: None,
        not_idempotent: false,
        function_name: "load_wan".to_string(),
    };

    registry.register(class_def, Arc::new(|_ctx, node, _node_id| {
        let model_name = node.inputs.get("model_name")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        let model_path = resolve_model_path("checkpoints", model_name);

        Box::pin(async move {
            let model_config = json!({
                "model_path": model_path,
            });
            let clip_config = json!({
                "type": "clip",
                "source_model": model_path,
            });
            let vae_config = json!({
                "type": "vae",
                "source_model": model_path,
            });
            Ok(vec![model_config, clip_config, vae_config])
        })
    }));
}

fn register_diffusion_model_loader(registry: &mut NodeRegistry) {
    let class_def = NodeClassDef {
        class_type: "DiffusionModelLoader".to_string(),
        display_name: "Load Diffusion Model (Standalone)".to_string(),
        category: "loaders".to_string(),
        input_types: NodeInputTypes {
            required: {
                let mut m = HashMap::new();
                m.insert("model_name".to_string(), InputTypeSpec {
                    type_name: "COMBO".to_string(),
                    extra: HashMap::new(),
                });
                m
            },
            optional: HashMap::new(),
            hidden: HashMap::new(),
        },
        output_types: vec![IoType::Model],
        output_names: vec!["MODEL".to_string()],
        output_is_list: vec![false],
        is_output_node: false,
        has_intermediate_output: false,
        is_changed: None,
        not_idempotent: false,
        function_name: "load_diffusion_model".to_string(),
    };

    registry.register(class_def, Arc::new(|_ctx, node, _node_id| {
        let model_name = node.inputs.get("model_name")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        let diffusion_model_path = resolve_model_path("diffusion_models", model_name);

        Box::pin(async move {
            Ok(vec![json!({
                "diffusion_model_path": diffusion_model_path,
            })])
        })
    }));
}

fn register_vae_loader(registry: &mut NodeRegistry) {
    let class_def = NodeClassDef {
        class_type: "VAELoader".to_string(),
        display_name: "Load VAE".to_string(),
        category: "loaders".to_string(),
        input_types: NodeInputTypes {
            required: {
                let mut m = HashMap::new();
                m.insert("vae_name".to_string(), InputTypeSpec {
                    type_name: "COMBO".to_string(),
                    extra: HashMap::new(),
                });
                m
            },
            optional: HashMap::new(),
            hidden: HashMap::new(),
        },
        output_types: vec![IoType::Vae],
        output_names: vec!["VAE".to_string()],
        output_is_list: vec![false],
        is_output_node: false,
        has_intermediate_output: false,
        is_changed: None,
        not_idempotent: false,
        function_name: "load_vae".to_string(),
    };

    registry.register(class_def, Arc::new(|_ctx, node, _node_id| {
        let vae_name = node.inputs.get("vae_name")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        let vae_path = resolve_model_path("vae", vae_name);

        Box::pin(async move {
            Ok(vec![json!({
                "type": "vae",
                "vae_path": vae_path,
            })])
        })
    }));
}

fn register_lora_loader(registry: &mut NodeRegistry) {
    let class_def = NodeClassDef {
        class_type: "LoraLoader".to_string(),
        display_name: "Load LoRA".to_string(),
        category: "loaders".to_string(),
        input_types: NodeInputTypes {
            required: {
                let mut m = HashMap::new();
                m.insert("model".to_string(), InputTypeSpec {
                    type_name: "MODEL".to_string(),
                    extra: HashMap::new(),
                });
                m.insert("clip".to_string(), InputTypeSpec {
                    type_name: "CLIP".to_string(),
                    extra: HashMap::new(),
                });
                m.insert("lora_name".to_string(), InputTypeSpec {
                    type_name: "COMBO".to_string(),
                    extra: HashMap::new(),
                });
                m.insert("strength_model".to_string(), InputTypeSpec {
                    type_name: "FLOAT".to_string(),
                    extra: HashMap::new(),
                });
                m.insert("strength_clip".to_string(), InputTypeSpec {
                    type_name: "FLOAT".to_string(),
                    extra: HashMap::new(),
                });
                m
            },
            optional: HashMap::new(),
            hidden: HashMap::new(),
        },
        output_types: vec![IoType::Model, IoType::Clip],
        output_names: vec!["MODEL".to_string(), "CLIP".to_string()],
        output_is_list: vec![false, false],
        is_output_node: false,
        has_intermediate_output: false,
        is_changed: None,
        not_idempotent: false,
        function_name: "load_lora".to_string(),
    };

    registry.register(class_def, Arc::new(|ctx, _node, node_id| {
        let model = ctx.resolve_input(node_id, "model")
            .unwrap_or_else(|_| json!({}));
        let clip = ctx.resolve_input(node_id, "clip")
            .unwrap_or_else(|_| json!({}));
        let lora_name = ctx.resolve_input(node_id, "lora_name")
            .unwrap_or_else(|_| json!(""))
            .as_str()
            .unwrap_or("")
            .to_string();
        let strength_model = ctx.resolve_input(node_id, "strength_model")
            .unwrap_or_else(|_| json!(1.0))
            .as_f64()
            .unwrap_or(1.0) as f32;
        let _strength_clip = ctx.resolve_input(node_id, "strength_clip")
            .unwrap_or_else(|_| json!(1.0))
            .as_f64()
            .unwrap_or(1.0) as f32;

        let lora_path = resolve_model_path("loras", &lora_name);

        Box::pin(async move {
            let mut model_out = model.as_object().cloned().unwrap_or_default();
            let loras = model_out.get("loras")
                .and_then(|v| v.as_array())
                .cloned()
                .unwrap_or_default();
            let mut loras = loras.clone();
            loras.push(json!({
                "path": lora_path,
                "multiplier": strength_model,
            }));
            model_out.insert("loras".to_string(), json!(loras));

            let mut clip_out = clip.as_object().cloned().unwrap_or_default();
            clip_out.insert("lora_path".to_string(), json!(lora_path));
            clip_out.insert("lora_strength".to_string(), json!(strength_model));

            Ok(vec![json!(model_out), json!(clip_out)])
        })
    }));
}

fn register_clip_text_encode(registry: &mut NodeRegistry) {
    let class_def = NodeClassDef {
        class_type: "CLIPTextEncode".to_string(),
        display_name: "CLIP Text Encode (Prompt)".to_string(),
        category: "conditioning".to_string(),
        input_types: NodeInputTypes {
            required: {
                let mut m = HashMap::new();
                m.insert("text".to_string(), InputTypeSpec {
                    type_name: "STRING".to_string(),
                    extra: HashMap::new(),
                });
                m.insert("clip".to_string(), InputTypeSpec {
                    type_name: "CLIP".to_string(),
                    extra: HashMap::new(),
                });
                m
            },
            optional: HashMap::new(),
            hidden: HashMap::new(),
        },
        output_types: vec![IoType::Conditioning],
        output_names: vec!["CONDITIONING".to_string()],
        output_is_list: vec![false],
        is_output_node: false,
        has_intermediate_output: false,
        is_changed: None,
        not_idempotent: false,
        function_name: "encode".to_string(),
    };

    registry.register(class_def, Arc::new(|ctx, _node, node_id| {
        let text = ctx.resolve_input(node_id, "text")
            .unwrap_or_else(|_| json!(""));
        let clip = ctx.resolve_input(node_id, "clip")
            .unwrap_or_else(|_| json!(null));

        Box::pin(async move {
            Ok(vec![
                json!({
                    "type": "conditioning",
                    "text": text,
                    "clip": clip,
                })
            ])
        })
    }));
}

fn register_ksampler(registry: &mut NodeRegistry) {
    let class_def = NodeClassDef {
        class_type: "KSampler".to_string(),
        display_name: "KSampler".to_string(),
        category: "sampling".to_string(),
        input_types: NodeInputTypes {
            required: {
                let mut m = HashMap::new();
                m.insert("model".to_string(), InputTypeSpec {
                    type_name: "MODEL".to_string(),
                    extra: HashMap::new(),
                });
                m.insert("seed".to_string(), InputTypeSpec {
                    type_name: "INT".to_string(),
                    extra: HashMap::new(),
                });
                m.insert("steps".to_string(), InputTypeSpec {
                    type_name: "INT".to_string(),
                    extra: HashMap::new(),
                });
                m.insert("cfg".to_string(), InputTypeSpec {
                    type_name: "FLOAT".to_string(),
                    extra: HashMap::new(),
                });
                m.insert("sampler_name".to_string(), InputTypeSpec {
                    type_name: "COMBO".to_string(),
                    extra: HashMap::new(),
                });
                m.insert("scheduler".to_string(), InputTypeSpec {
                    type_name: "COMBO".to_string(),
                    extra: HashMap::new(),
                });
                m.insert("positive".to_string(), InputTypeSpec {
                    type_name: "CONDITIONING".to_string(),
                    extra: HashMap::new(),
                });
                m.insert("negative".to_string(), InputTypeSpec {
                    type_name: "CONDITIONING".to_string(),
                    extra: HashMap::new(),
                });
                m.insert("latent_image".to_string(), InputTypeSpec {
                    type_name: "LATENT".to_string(),
                    extra: HashMap::new(),
                });
                m
            },
            optional: {
                let mut m = HashMap::new();
                m.insert("vae".to_string(), InputTypeSpec {
                    type_name: "VAE".to_string(),
                    extra: HashMap::new(),
                });
                m
            },
            hidden: HashMap::new(),
        },
        output_types: vec![IoType::Latent],
        output_names: vec!["LATENT".to_string()],
        output_is_list: vec![false],
        is_output_node: false,
        has_intermediate_output: false,
        is_changed: None,
        not_idempotent: true,
        function_name: "sample".to_string(),
    };

    registry.register(class_def, Arc::new(|ctx, _node, node_id| {
        let model = ctx.resolve_input(node_id, "model").unwrap_or_else(|_| json!({}));
        let seed = ctx.resolve_input(node_id, "seed").unwrap_or_else(|_| json!(42));
        let steps = ctx.resolve_input(node_id, "steps").unwrap_or_else(|_| json!(20));
        let cfg = ctx.resolve_input(node_id, "cfg").unwrap_or_else(|_| json!(7.0));
        let sampler_name = ctx.resolve_input(node_id, "sampler_name")
            .unwrap_or_else(|_| json!("euler_ancestral"));
        let scheduler = ctx.resolve_input(node_id, "scheduler")
            .unwrap_or_else(|_| json!("normal"));
        let positive = ctx.resolve_input(node_id, "positive").unwrap_or_else(|_| json!(null));
        let negative = ctx.resolve_input(node_id, "negative").unwrap_or_else(|_| json!(null));
        let latent_image = ctx.resolve_input(node_id, "latent_image").unwrap_or_else(|_| json!(null));
        let vae = ctx.resolve_input(node_id, "vae").ok();

        let backend = ctx.backend();
        let supports_img_gen = backend.supports_image_generation();

        Box::pin(async move {
            if !supports_img_gen {
                tracing::warn!(
                    "KSampler: backend does not support image generation, skipping inference. \
                     Check that sd-cli or local inference backend is properly configured."
                );
            }
            if supports_img_gen {
                let prompt_text = positive.get("text")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let neg_prompt_text = negative.get("text")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");

                let sample_method = parse_sample_method(
                    sampler_name.as_str().unwrap_or("euler_ancestral")
                );
                let sched = parse_scheduler(
                    scheduler.as_str().unwrap_or("normal")
                );

                let mut model_config = ModelConfig::new();

                if let Some(path) = model.get("model_path").and_then(|v| v.as_str()) {
                    model_config = model_config.with_model(path);
                }
                if let Some(path) = model.get("diffusion_model_path").and_then(|v| v.as_str()) {
                    model_config = model_config.with_diffusion_model(path);
                }
                if let Some(path) = model.get("clip_l_path").and_then(|v| v.as_str()) {
                    model_config = model_config.with_clip_l(path);
                }
                if let Some(path) = model.get("clip_g_path").and_then(|v| v.as_str()) {
                    model_config = model_config.with_clip_g(path);
                }
                if let Some(path) = model.get("t5xxl_path").and_then(|v| v.as_str()) {
                    model_config = model_config.with_t5xxl(path);
                }
                if let Some(path) = model.get("llm_path").and_then(|v| v.as_str()) {
                    model_config = model_config.with_llm(path);
                }
                if let Some(path) = model.get("llm_vision_path").and_then(|v| v.as_str()) {
                    model_config = model_config.with_llm_vision(path);
                }
                if let Some(path) = model.get("clip_vision_path").and_then(|v| v.as_str()) {
                    model_config = model_config.with_clip_vision(path);
                }
                if let Some(path) = model.get("control_net_path").and_then(|v| v.as_str()) {
                    model_config = model_config.with_control_net(path);
                }

                for cond in [&positive, &negative] {
                    if let Some(clip) = cond.get("clip") {
                        if let Some(path) = clip.get("clip_l_path").and_then(|v| v.as_str()) {
                            model_config = model_config.with_clip_l(path);
                        }
                        if let Some(path) = clip.get("clip_g_path").and_then(|v| v.as_str()) {
                            model_config = model_config.with_clip_g(path);
                        }
                        if let Some(path) = clip.get("t5xxl_path").and_then(|v| v.as_str()) {
                            model_config = model_config.with_t5xxl(path);
                        }
                    }
                }

                if let Some(vae_val) = &vae {
                    if let Some(path) = vae_val.get("vae_path").and_then(|v| v.as_str()) {
                        model_config = model_config.with_vae(path);
                    }
                }
                if let Some(path) = model.get("vae_path").and_then(|v| v.as_str()) {
                    model_config = model_config.with_vae(path);
                }

                let needs_clip_auto_detect = model_config.clip_l_path.is_none()
                    || model_config.clip_g_path.is_none()
                    || model_config.t5xxl_path.is_none();
                let needs_vae_auto_detect = model_config.vae_path.is_none();
                if needs_clip_auto_detect || needs_vae_auto_detect {
                    let model_type_str = model.get("model_type")
                        .and_then(|v| v.as_str())
                        .unwrap_or("");
                    let detected_type = match model_type_str {
                        "sd3" => ModelType::SD3,
                        "flux" => ModelType::Flux,
                        "sdxl" => ModelType::SDXL,
                        "sd15" => ModelType::SD15,
                        "wan" => ModelType::Wan,
                        _ => {
                            let ckpt = model.get("model_path")
                                .or_else(|| model.get("diffusion_model_path"))
                                .and_then(|v| v.as_str())
                                .unwrap_or("");
                            detect_model_type(ckpt)
                        }
                    };

                    if needs_clip_auto_detect {
                        let (clip_l, clip_g, t5xxl) = auto_detect_text_encoders(detected_type);
                        if model_config.clip_l_path.is_none() {
                            if let Some(path) = clip_l {
                                model_config = model_config.with_clip_l(path);
                            }
                        }
                        if model_config.clip_g_path.is_none() {
                            if let Some(path) = clip_g {
                                model_config = model_config.with_clip_g(path);
                            }
                        }
                        if model_config.t5xxl_path.is_none() {
                            if let Some(path) = t5xxl {
                                model_config = model_config.with_t5xxl(path);
                            }
                        }
                        tracing::info!(
                            "KSampler: auto-detected text encoders for {:?} model: clip_l={:?}, clip_g={:?}, t5xxl={:?}",
                            detected_type, model_config.clip_l_path, model_config.clip_g_path, model_config.t5xxl_path
                        );
                    }

                    if needs_vae_auto_detect {
                        if let Some(path) = auto_detect_vae(detected_type) {
                            model_config = model_config.with_vae(path);
                            tracing::info!(
                                "KSampler: auto-detected vae for {:?} model: vae={:?}",
                                detected_type, model_config.vae_path
                            );
                        }
                    }
                }

                let mut width = 512i32;
                let mut height = 512i32;
                if let Some(latent) = latent_image.as_object() {
                    if let Some(w) = latent.get("width").and_then(|v| v.as_i64()) {
                        width = w as i32;
                    }
                    if let Some(h) = latent.get("height").and_then(|v| v.as_i64()) {
                        height = h as i32;
                    }
                }

                let mut params = ImageGenParams::new(prompt_text)
                    .with_negative_prompt(neg_prompt_text)
                    .with_seed(seed.as_i64().unwrap_or(42))
                    .with_sample_steps(steps.as_i64().unwrap_or(20) as i32)
                    .with_cfg_scale(cfg.as_f64().unwrap_or(7.0) as f32)
                    .with_sample_method(sample_method)
                    .with_scheduler(sched)
                    .with_dimensions(width, height)
                    .with_model_config(model_config);

                if let Some(loras) = model.get("loras").and_then(|v| v.as_array()) {
                    for lora in loras {
                        if let (Some(path), Some(mult)) = (
                            lora.get("path").and_then(|v| v.as_str()),
                            lora.get("multiplier").and_then(|v| v.as_f64()),
                        ) {
                            params = params.with_lora(path, mult as f32);
                        }
                    }
                }

                if let Some(cn) = positive.get("control_net") {
                    if let Some(cn_path) = cn.get("path").and_then(|v| v.as_str()) {
                        if !cn_path.is_empty() {
                            params.model_config.control_net_path = Some(cn_path.to_string());
                        }
                    }
                }
                if let Some(cn_image) = positive.get("control_image") {
                    if let Ok(sd_img) = serde_json::from_value::<comfy_inference::SdImage>(cn_image.clone()) {
                        params.control_image = Some(sd_img);
                    } else if let Some(img_obj) = cn_image.as_object() {
                        if let Some(images) = img_obj.get("images").and_then(|v| v.as_array()) {
                            if let Some(first) = images.first() {
                                if let Ok(sd_img) = serde_json::from_value::<comfy_inference::SdImage>(first.clone()) {
                                    params.control_image = Some(sd_img);
                                }
                            }
                        }
                    }
                }
                if let Some(cn_strength) = positive.get("control_strength") {
                    params.control_strength = cn_strength.as_f64().unwrap_or(0.9) as f32;
                }

                let model_type_str = model.get("model_type")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let detected_type = match model_type_str {
                    "sd3" => ModelType::SD3,
                    "flux" => ModelType::Flux,
                    "sdxl" => ModelType::SDXL,
                    "sd15" => ModelType::SD15,
                    "wan" => ModelType::Wan,
                    _ => {
                        let ckpt = model.get("model_path")
                            .or_else(|| model.get("diffusion_model_path"))
                            .and_then(|v| v.as_str())
                            .unwrap_or("");
                        detect_model_type(ckpt)
                    }
                };
                let (need_clip_l, need_clip_g, need_t5xxl) = match detected_type {
                    ModelType::SD3 => (true, true, true),
                    ModelType::Flux => (true, false, true),
                    ModelType::SDXL => (true, true, false),
                    ModelType::SD15 => (true, false, false),
                    ModelType::Wan => (false, false, true),
                    ModelType::Unknown => (false, false, false),
                };
                let mut missing_encoders = Vec::new();
                if need_clip_l && params.model_config.clip_l_path.is_none() {
                    missing_encoders.push("clip_l");
                }
                if need_clip_g && params.model_config.clip_g_path.is_none() {
                    missing_encoders.push("clip_g");
                }
                if need_t5xxl && params.model_config.t5xxl_path.is_none() {
                    missing_encoders.push("t5xxl");
                }
                if !missing_encoders.is_empty() {
                    tracing::error!(
                        "KSampler: {:?} model requires text encoders [{}] but they are missing. \
                         Please download them to models/text_encoders/ directory.",
                        detected_type,
                        missing_encoders.join(", ")
                    );
                    return Err(ExecutorError::NodeExecutionFailed {
                        node_id: node_id.to_string(),
                        message: format!(
                            "{:?} model requires text encoders [{}] but they are missing. \
                             Please download them to models/text_encoders/ directory.",
                            detected_type,
                            missing_encoders.join(", ")
                        ),
                    });
                }

                match backend.generate_image(params) {
                    Ok(images) => {
                        let image_data: Vec<Value> = images.iter().map(|img| {
                            serde_json::to_value(img).unwrap_or_else(|_| json!({
                                "type": "image",
                                "width": img.width,
                                "height": img.height,
                                "channel": img.channel,
                            }))
                        }).collect();
                        Ok(vec![json!({
                            "type": "latent",
                            "samples": image_data,
                            "seed": seed,
                            "decoded_images": images.len(),
                        })])
                    }
                    Err(e) => Err(ExecutorError::Inference(e)),
                }
            } else {
                Ok(vec![json!({
                    "type": "latent",
                    "model": model,
                    "seed": seed,
                    "steps": steps,
                    "cfg": cfg,
                    "sampler": sampler_name,
                    "scheduler": scheduler,
                    "positive": positive,
                    "negative": negative,
                    "latent_image": latent_image,
                })])
            }
        })
    }));
}

fn register_load_audio(registry: &mut NodeRegistry) {
    let class_def = NodeClassDef {
        class_type: "LoadAudio".to_string(),
        display_name: "Load Audio".to_string(),
        category: "audio".to_string(),
        input_types: NodeInputTypes {
            required: {
                let mut m = HashMap::new();
                m.insert("audio".to_string(), InputTypeSpec {
                    type_name: "COMBO".to_string(),
                    extra: HashMap::new(),
                });
                m
            },
            optional: HashMap::new(),
            hidden: HashMap::new(),
        },
        output_types: vec![IoType::Audio],
        output_names: vec!["AUDIO".to_string()],
        output_is_list: vec![false],
        is_output_node: false,
        has_intermediate_output: false,
        is_changed: None,
        not_idempotent: false,
        function_name: "load".to_string(),
    };

    registry.register(class_def, Arc::new(|_ctx, node, node_id| {
        let audio_name = node.inputs.get("audio")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        let input_dir = std::env::var("COMFY_INPUT_DIR")
            .unwrap_or_else(|_| "input".to_string());
        let audio_path = std::path::Path::new(&input_dir).join(audio_name);

        let audio_path_str = audio_path.to_string_lossy().to_string();
        let filename = audio_name.to_string();

        Box::pin(async move {
            if !std::path::Path::new(&audio_path_str).exists() {
                return Err(ExecutorError::NodeExecutionFailed {
                    node_id: node_id.to_string(),
                    message: format!("Audio file not found: {}", audio_path_str),
                });
            }

            let ext = std::path::Path::new(&filename)
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("")
                .to_lowercase();

            let duration_secs = get_audio_duration(&audio_path_str);

            Ok(vec![json!({
                "type": "audio",
                "path": audio_path_str,
                "filename": filename,
                "format": ext,
                "duration": duration_secs,
            })])
        })
    }));
}

fn register_save_audio(registry: &mut NodeRegistry) {
    let class_def = NodeClassDef {
        class_type: "SaveAudio".to_string(),
        display_name: "Save Audio".to_string(),
        category: "audio".to_string(),
        input_types: NodeInputTypes {
            required: {
                let mut m = HashMap::new();
                m.insert("audio".to_string(), InputTypeSpec {
                    type_name: "AUDIO".to_string(),
                    extra: HashMap::new(),
                });
                m
            },
            optional: {
                let mut m = HashMap::new();
                m.insert("filename_prefix".to_string(), InputTypeSpec {
                    type_name: "STRING".to_string(),
                    extra: HashMap::new(),
                });
                m.insert("format".to_string(), InputTypeSpec {
                    type_name: "COMBO".to_string(),
                    extra: HashMap::new(),
                });
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
        not_idempotent: false,
        function_name: "save".to_string(),
    };

    registry.register(class_def, Arc::new(|ctx, node, node_id| {
        let audio_val = ctx.resolve_input(node_id, "audio")
            .unwrap_or_else(|_| json!(null));
        let prefix = node.inputs.get("filename_prefix")
            .and_then(|v| v.as_str())
            .unwrap_or("audio");
        let format = node.inputs.get("format")
            .and_then(|v| v.as_str())
            .unwrap_or("wav");

        let output_dir = std::env::var("COMFY_OUTPUT_DIR")
            .unwrap_or_else(|_| "output".to_string());
        let output_dir_path = std::path::PathBuf::from(&output_dir);
        if !output_dir_path.exists() {
            let _ = std::fs::create_dir_all(&output_dir_path);
        }

        Box::pin(async move {
            let src_path = audio_val.get("path")
                .and_then(|v| v.as_str())
                .unwrap_or("");

            if src_path.is_empty() {
                return Err(ExecutorError::NodeExecutionFailed {
                    node_id: node_id.to_string(),
                    message: "No audio input provided".to_string(),
                });
            }

            let ext = match format {
                "mp3" => "mp3",
                "flac" => "flac",
                "ogg" => "ogg",
                _ => "wav",
            };

            let filename = format!("{}_{}.{}", prefix, chrono::Utc::now().format("%Y%m%d_%H%M%S"), ext);
            let dest_path = output_dir_path.join(&filename);

            if ext == "wav" {
                std::fs::copy(src_path, &dest_path)
                    .map_err(|e| ExecutorError::NodeExecutionFailed {
                        node_id: node_id.to_string(),
                        message: format!("Failed to save audio: {}", e),
                    })?;
            } else if is_ffmpeg_available() {
                let status = tokio::process::Command::new("ffmpeg")
                    .arg("-y")
                    .arg("-i").arg(src_path)
                    .arg(&dest_path)
                    .stdout(std::process::Stdio::null())
                    .stderr(std::process::Stdio::null())
                    .status()
                    .await
                    .map_err(|e| ExecutorError::NodeExecutionFailed {
                        node_id: node_id.to_string(),
                        message: format!("Failed to run ffmpeg: {}", e),
                    })?;

                if !status.success() {
                    return Err(ExecutorError::NodeExecutionFailed {
                        node_id: node_id.to_string(),
                        message: "ffmpeg conversion failed".to_string(),
                    });
                }
            } else {
                std::fs::copy(src_path, &dest_path)
                    .map_err(|e| ExecutorError::NodeExecutionFailed {
                        node_id: node_id.to_string(),
                        message: format!("FFmpeg not available, failed to copy audio: {}", e),
                    })?;
            }

            Ok(vec![json!({
                "type": "audio",
                "path": dest_path.to_string_lossy().to_string(),
                "filename": filename,
                "format": ext,
                "audios": [{
                    "filename": filename,
                    "subfolder": "",
                    "type": "output",
                }],
            })])
        })
    }));
}

fn register_audio_to_llm(registry: &mut NodeRegistry) {
    let class_def = NodeClassDef {
        class_type: "AudioToLLM".to_string(),
        display_name: "Audio to LLM".to_string(),
        category: "audio".to_string(),
        input_types: NodeInputTypes {
            required: {
                let mut m = HashMap::new();
                m.insert("audio".to_string(), InputTypeSpec {
                    type_name: "AUDIO".to_string(),
                    extra: HashMap::new(),
                });
                m.insert("llm".to_string(), InputTypeSpec {
                    type_name: "LLM".to_string(),
                    extra: HashMap::new(),
                });
                m.insert("prompt".to_string(), InputTypeSpec {
                    type_name: "STRING".to_string(),
                    extra: HashMap::new(),
                });
                m
            },
            optional: {
                let mut m = HashMap::new();
                m.insert("max_tokens".to_string(), InputTypeSpec {
                    type_name: "INT".to_string(),
                    extra: HashMap::new(),
                });
                m.insert("temperature".to_string(), InputTypeSpec {
                    type_name: "FLOAT".to_string(),
                    extra: HashMap::new(),
                });
                m
            },
            hidden: HashMap::new(),
        },
        output_types: vec![IoType::String],
        output_names: vec!["STRING".to_string()],
        output_is_list: vec![false],
        is_output_node: false,
        has_intermediate_output: false,
        is_changed: None,
        not_idempotent: true,
        function_name: "process".to_string(),
    };

    registry.register(class_def, Arc::new(|ctx, node, node_id| {
        let audio = ctx.resolve_input(node_id, "audio")
            .unwrap_or_else(|_| json!(null));
        let llm = ctx.resolve_input(node_id, "llm")
            .unwrap_or_else(|_| json!(null));
        let prompt = ctx.resolve_input(node_id, "prompt")
            .unwrap_or_else(|_| json!(""));
        let max_tokens = node.inputs.get("max_tokens")
            .and_then(|v| v.as_i64())
            .unwrap_or(512);
        let temperature = node.inputs.get("temperature")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.7);

        let llm_config = ctx.get_extra_data("llm_config")
            .cloned()
            .unwrap_or(json!({
                "mode": "local",
                "cli_path": "/home/acproject/workspace/rust_projects/comfyui-rust/cpp/llama.cpp-qwen3-omni/build/bin/llama-cli",
            }));

        let model_path = llm.get("model_path")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let audio_path = audio.get("path")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let prompt_text = prompt.as_str().unwrap_or("").to_string();

        Box::pin(async move {
            if audio_path.is_empty() {
                return Err(ExecutorError::NodeExecutionFailed {
                    node_id: node_id.to_string(),
                    message: "No audio input provided".to_string(),
                });
            }

            if model_path.is_empty() {
                return Err(ExecutorError::NodeExecutionFailed {
                    node_id: node_id.to_string(),
                    message: "No LLM model loaded".to_string(),
                });
            }

            let cli_path = llm_config.get("cli_path")
                .and_then(|v| v.as_str())
                .unwrap_or("/home/acproject/workspace/rust_projects/comfyui-rust/cpp/llama.cpp-qwen3-omni/build/bin/llama-cli")
                .to_string();

            let mut cmd = tokio::process::Command::new(&cli_path);
            cmd.arg("-m").arg(&model_path)
                .arg("--audio").arg(&audio_path)
                .arg("-p").arg(&prompt_text)
                .arg("--n-predict").arg(max_tokens.to_string())
                .arg("--temp").arg(temperature.to_string())
                .arg("--no-display-prompt")
                .arg("--log-disable")
                .stdout(std::process::Stdio::piped())
                .stderr(std::process::Stdio::piped());

            if let Some(mmproj) = llm.get("mmproj_path").and_then(|v| v.as_str()) {
                if !mmproj.is_empty() {
                    cmd.arg("--mmproj").arg(mmproj);
                }
            }

            if let Some(extra_args) = llm_config.get("extra_args").and_then(|v| v.as_str()) {
                for arg in extra_args.split_whitespace() {
                    cmd.arg(arg);
                }
            }

            match cmd.output().await {
                Ok(output) => {
                    if output.status.success() {
                        let text = String::from_utf8_lossy(&output.stdout).to_string();
                        Ok(vec![json!(text.trim())])
                    } else {
                        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
                        Err(ExecutorError::NodeExecutionFailed {
                            node_id: node_id.to_string(),
                            message: format!("llama-cli audio processing failed: {}", stderr),
                        })
                    }
                }
                Err(e) => {
                    Err(ExecutorError::NodeExecutionFailed {
                        node_id: node_id.to_string(),
                        message: format!("Failed to execute llama-cli: {}", e),
                    })
                }
            }
        })
    }));
}

fn get_audio_duration(path: &str) -> f64 {
    if is_ffmpeg_available() {
        let output = std::process::Command::new("ffprobe")
            .arg("-v").arg("quiet")
            .arg("-show_entries").arg("format=duration")
            .arg("-of").arg("default=noprint_wrappers=1:nokey=1")
            .arg(path)
            .output();
        if let Ok(out) = output {
            let s = String::from_utf8_lossy(&out.stdout).trim().to_string();
            if let Ok(d) = s.parse::<f64>() {
                return d;
            }
        }
    }
    0.0
}

fn is_ffmpeg_available() -> bool {
    std::process::Command::new("ffmpeg")
        .arg("-version")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}
fn register_ltx_loader(registry: &mut NodeRegistry) {
    let class_def = NodeClassDef {
        class_type: "LTXLoader".to_string(),
        display_name: "Load LTX Model".to_string(),
        category: "loaders".to_string(),
        input_types: NodeInputTypes {
            required: {
                let mut m = HashMap::new();
                m.insert("model_name".to_string(), InputTypeSpec {
                    type_name: "COMBO".to_string(),
                    extra: HashMap::new(),
                });
                m
            },
            optional: HashMap::new(),
            hidden: HashMap::new(),
        },
        output_types: vec![IoType::Model, IoType::Clip, IoType::Vae],
        output_names: vec!["MODEL".to_string(), "CLIP".to_string(), "VAE".to_string()],
        output_is_list: vec![false, false, false],
        is_output_node: false,
        has_intermediate_output: false,
        is_changed: None,
        not_idempotent: false,
        function_name: "load_ltx".to_string(),
    };

    registry.register(class_def, Arc::new(|_ctx, node, _node_id| {
        let model_name = node.inputs.get("model_name")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        let model_path = resolve_model_path("checkpoints", model_name);

        Box::pin(async move {
            let model_config = json!({
                "model_path": model_path,
                "model_type": "ltx",
            });
            let clip_config = json!({
                "type": "clip",
                "source_model": model_path,
            });
            let vae_config = json!({
                "type": "vae",
                "source_model": model_path,
            });
            Ok(vec![model_config, clip_config, vae_config])
        })
    }));
}

fn register_ltx_video_sampler(registry: &mut NodeRegistry) {
    let class_def = NodeClassDef {
        class_type: "LTXVideoSampler".to_string(),
        display_name: "LTX Video Sampler".to_string(),
        category: "sampling/video".to_string(),
        input_types: NodeInputTypes {
            required: {
                let mut m = HashMap::new();
                m.insert("model".to_string(), InputTypeSpec {
                    type_name: "MODEL".to_string(),
                    extra: HashMap::new(),
                });
                m.insert("seed".to_string(), InputTypeSpec {
                    type_name: "INT".to_string(),
                    extra: HashMap::new(),
                });
                m.insert("steps".to_string(), InputTypeSpec {
                    type_name: "INT".to_string(),
                    extra: HashMap::new(),
                });
                m.insert("cfg".to_string(), InputTypeSpec {
                    type_name: "FLOAT".to_string(),
                    extra: HashMap::new(),
                });
                m.insert("sampler_name".to_string(), InputTypeSpec {
                    type_name: "COMBO".to_string(),
                    extra: HashMap::new(),
                });
                m.insert("scheduler".to_string(), InputTypeSpec {
                    type_name: "COMBO".to_string(),
                    extra: HashMap::new(),
                });
                m.insert("positive".to_string(), InputTypeSpec {
                    type_name: "CONDITIONING".to_string(),
                    extra: HashMap::new(),
                });
                m.insert("negative".to_string(), InputTypeSpec {
                    type_name: "CONDITIONING".to_string(),
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
                m.insert("video_frames".to_string(), InputTypeSpec {
                    type_name: "INT".to_string(),
                    extra: HashMap::new(),
                });
                m.insert("num_frames_per_seed".to_string(), InputTypeSpec {
                    type_name: "INT".to_string(),
                    extra: HashMap::new(),
                });
                m
            },
            optional: {
                let mut m = HashMap::new();
                m.insert("init_image".to_string(), InputTypeSpec {
                    type_name: "IMAGE".to_string(),
                    extra: HashMap::new(),
                });
                m.insert("flow_shift".to_string(), InputTypeSpec {
                    type_name: "FLOAT".to_string(),
                    extra: HashMap::new(),
                });
                m
            },
            hidden: HashMap::new(),
        },
        output_types: vec![IoType::Latent],
        output_names: vec!["LATENT".to_string()],
        output_is_list: vec![false],
        is_output_node: false,
        has_intermediate_output: false,
        is_changed: None,
        not_idempotent: true,
        function_name: "sample_video".to_string(),
    };

    registry.register(class_def, Arc::new(|ctx, _node, node_id| {
        let model = ctx.resolve_input(node_id, "model").unwrap_or_else(|_| json!({}));
        let seed = ctx.resolve_input(node_id, "seed").unwrap_or_else(|_| json!(42));
        let steps = ctx.resolve_input(node_id, "steps").unwrap_or_else(|_| json!(20));
        let cfg = ctx.resolve_input(node_id, "cfg").unwrap_or_else(|_| json!(3.0));
        let sampler_name = ctx.resolve_input(node_id, "sampler_name")
            .unwrap_or_else(|_| json!("euler"));
        let scheduler = ctx.resolve_input(node_id, "scheduler")
            .unwrap_or_else(|_| json!("normal"));
        let positive = ctx.resolve_input(node_id, "positive").unwrap_or_else(|_| json!(null));
        let negative = ctx.resolve_input(node_id, "negative").unwrap_or_else(|_| json!(null));
        let width = ctx.resolve_input(node_id, "width").unwrap_or_else(|_| json!(768));
        let height = ctx.resolve_input(node_id, "height").unwrap_or_else(|_| json!(512));
        let video_frames = ctx.resolve_input(node_id, "video_frames").unwrap_or_else(|_| json!(97));
        let num_frames_per_seed = ctx.resolve_input(node_id, "num_frames_per_seed")
            .unwrap_or_else(|_| json!(1));
        let _init_image = ctx.resolve_input(node_id, "init_image").ok();
        let flow_shift = ctx.resolve_input(node_id, "flow_shift")
            .ok()
            .and_then(|v| v.as_f64());

        let backend = ctx.backend();
        let supports_vid_gen = backend.supports_video_generation();

        Box::pin(async move {
            if supports_vid_gen {
                let prompt_text = positive.get("text")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let negative_text = negative.get("text")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");

                let mut model_config = ModelConfig::default();
                if let Some(path) = model.get("model_path").and_then(|v| v.as_str()) {
                    model_config = model_config.with_model(path);
                }
                if let Some(path) = model.get("diffusion_model_path").and_then(|v| v.as_str()) {
                    model_config = model_config.with_diffusion_model(path);
                }
                if let Some(path) = model.get("vae_path").and_then(|v| v.as_str()) {
                    model_config = model_config.with_vae(path);
                }
                if let Some(path) = model.get("clip_l_path").and_then(|v| v.as_str()) {
                    model_config = model_config.with_clip_l(path);
                }
                if let Some(path) = model.get("clip_g_path").and_then(|v| v.as_str()) {
                    model_config = model_config.with_clip_g(path);
                }
                if let Some(path) = model.get("clip_vision_path").and_then(|v| v.as_str()) {
                    model_config = model_config.with_clip_vision(path);
                }
                if let Some(path) = model.get("t5xxl_path").and_then(|v| v.as_str()) {
                    model_config = model_config.with_t5xxl(path);
                }

                let clip_config = positive.get("clip");
                if let Some(clip) = clip_config {
                    if model_config.clip_l_path.is_none() {
                        if let Some(path) = clip.get("clip_l_path").and_then(|v| v.as_str()) {
                            model_config = model_config.with_clip_l(path);
                        }
                    }
                    if model_config.clip_g_path.is_none() {
                        if let Some(path) = clip.get("clip_g_path").and_then(|v| v.as_str()) {
                            model_config = model_config.with_clip_g(path);
                        }
                    }
                    if model_config.t5xxl_path.is_none() {
                        if let Some(path) = clip.get("t5xxl_path").and_then(|v| v.as_str()) {
                            model_config = model_config.with_t5xxl(path);
                        }
                    }
                }

                let needs_clip_auto_detect = model_config.clip_l_path.is_none()
                    || model_config.t5xxl_path.is_none();
                let needs_vae_auto_detect = model_config.vae_path.is_none();
                if needs_clip_auto_detect || needs_vae_auto_detect {
                    if needs_clip_auto_detect {
                        let (clip_l, _, t5xxl) = auto_detect_text_encoders(ModelType::Flux);
                        if model_config.clip_l_path.is_none() {
                            if let Some(path) = clip_l {
                                model_config = model_config.with_clip_l(path);
                            }
                        }
                        if model_config.t5xxl_path.is_none() {
                            if let Some(path) = t5xxl {
                                model_config = model_config.with_t5xxl(path);
                            }
                        }
                    }

                    if needs_vae_auto_detect {
                        let base = get_models_base_dir();
                        let vae_dir = base.join("vae");
                        if let Some(path) = find_file_in_dir(&vae_dir, &["ltx_vae", "ae"]) {
                            model_config = model_config.with_vae(path);
                        } else if let Some(path) = auto_detect_vae(ModelType::Flux) {
                            model_config = model_config.with_vae(path);
                        }
                    }
                }

                let sample_method = parse_sample_method(
                    sampler_name.as_str().unwrap_or("euler")
                );
                let scheduler_type = parse_scheduler(
                    scheduler.as_str().unwrap_or("normal")
                );

                let mut video_params = comfy_inference::VideoGenParams::new(prompt_text)
                    .with_negative_prompt(negative_text)
                    .with_dimensions(
                        width.as_i64().unwrap_or(768) as i32,
                        height.as_i64().unwrap_or(512) as i32,
                    )
                    .with_seed(seed.as_i64().unwrap_or(42))
                    .with_video_frames(video_frames.as_i64().unwrap_or(97) as i32)
                    .with_model_config(model_config);

                video_params.sample_params.sample_steps = steps.as_i64().unwrap_or(20) as i32;
                video_params.sample_params.guidance.txt_cfg = cfg.as_f64().unwrap_or(3.0) as f32;
                video_params.sample_params.sample_method = sample_method;
                video_params.sample_params.scheduler = scheduler_type;
                video_params.sample_params.flow_shift = flow_shift.map(|v| v as f32);

                match backend.generate_video(video_params) {
                    Ok(video) => {
                        let frame_count = video.frame_count();
                        tracing::info!("LTXVideoSampler: generated {} video frames", frame_count);
                        Ok(vec![json!({
                            "type": "video",
                            "frame_count": frame_count,
                            "fps": video.fps,
                            "num_frames_per_seed": num_frames_per_seed,
                        })])
                    }
                    Err(e) => {
                        tracing::error!("LTX video generation failed: {}", e);
                        Err(ExecutorError::Inference(e))
                    }
                }
            } else {
                Ok(vec![json!({
                    "type": "video",
                    "seed": seed,
                    "steps": steps,
                    "cfg": cfg,
                    "sampler": sampler_name,
                    "scheduler": scheduler,
                    "positive": positive,
                    "negative": negative,
                    "width": width,
                    "height": height,
                    "video_frames": video_frames,
                    "num_frames_per_seed": num_frames_per_seed,
                })])
            }
        })
    }));
}

fn register_llm_loader(registry: &mut NodeRegistry) {
    let class_def = NodeClassDef {
        class_type: "LLMLoader".to_string(),
        display_name: "Load LLM Model".to_string(),
        category: "loaders/llm".to_string(),
        input_types: NodeInputTypes {
            required: {
                let mut m = HashMap::new();
                m.insert("llm_model_name".to_string(), InputTypeSpec {
                    type_name: "COMBO".to_string(),
                    extra: HashMap::new(),
                });
                m
            },
            optional: HashMap::new(),
            hidden: HashMap::new(),
        },
        output_types: vec![IoType::Llm],
        output_names: vec!["LLM".to_string()],
        output_is_list: vec![false],
        is_output_node: false,
        has_intermediate_output: false,
        is_changed: None,
        not_idempotent: false,
        function_name: "load_llm".to_string(),
    };

    registry.register(class_def, Arc::new(|_ctx, node, _node_id| {
        let model_name = node.inputs.get("llm_model_name")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        let model_path = resolve_model_path("llm", model_name);

        let mmproj_path = {
            let model_file = std::path::Path::new(&model_path);
            let model_dir = model_file.parent().unwrap_or(std::path::Path::new("."));
            let mut found: Option<String> = None;
            if let Ok(entries) = std::fs::read_dir(model_dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                        if name.to_lowercase().starts_with("mmproj-") && name.to_lowercase().ends_with(".gguf") {
                            found = Some(path.to_string_lossy().to_string());
                            break;
                        }
                    }
                }
            }
            found
        };

        Box::pin(async move {
            let mut result = json!({
                "type": "llm",
                "model_path": model_path,
                "model_name": model_name,
            });
            if let Some(mmproj) = mmproj_path {
                result.as_object_mut().unwrap().insert("mmproj_path".to_string(), json!(mmproj));
            }
            Ok(vec![result])
        })
    }));
}

fn register_llm_text_gen(registry: &mut NodeRegistry) {
    let class_def = NodeClassDef {
        class_type: "LLMTextGen".to_string(),
        display_name: "LLM Text Generation".to_string(),
        category: "llm".to_string(),
        input_types: NodeInputTypes {
            required: {
                let mut m = HashMap::new();
                m.insert("llm".to_string(), InputTypeSpec {
                    type_name: "LLM".to_string(),
                    extra: HashMap::new(),
                });
                m.insert("prompt".to_string(), InputTypeSpec {
                    type_name: "STRING".to_string(),
                    extra: HashMap::new(),
                });
                m
            },
            optional: {
                let mut m = HashMap::new();
                m.insert("system_prompt".to_string(), InputTypeSpec {
                    type_name: "STRING".to_string(),
                    extra: HashMap::new(),
                });
                m.insert("max_tokens".to_string(), InputTypeSpec {
                    type_name: "INT".to_string(),
                    extra: HashMap::new(),
                });
                m.insert("temperature".to_string(), InputTypeSpec {
                    type_name: "FLOAT".to_string(),
                    extra: HashMap::new(),
                });
                m.insert("top_p".to_string(), InputTypeSpec {
                    type_name: "FLOAT".to_string(),
                    extra: HashMap::new(),
                });
                m.insert("seed".to_string(), InputTypeSpec {
                    type_name: "INT".to_string(),
                    extra: HashMap::new(),
                });
                m
            },
            hidden: HashMap::new(),
        },
        output_types: vec![IoType::String],
        output_names: vec!["STRING".to_string()],
        output_is_list: vec![false],
        is_output_node: false,
        has_intermediate_output: false,
        is_changed: None,
        not_idempotent: true,
        function_name: "generate".to_string(),
    };

    registry.register(class_def, Arc::new(|ctx, node, node_id| {
        let llm = ctx.resolve_input(node_id, "llm")
            .unwrap_or_else(|_| json!(null));
        let prompt = ctx.resolve_input(node_id, "prompt")
            .unwrap_or_else(|_| json!(""));
        let system_prompt = ctx.resolve_input(node_id, "system_prompt")
            .ok()
            .and_then(|v| v.as_str().map(|s| s.to_string()));
        let max_tokens = node.inputs.get("max_tokens")
            .and_then(|v| v.as_i64())
            .unwrap_or(512);
        let temperature = node.inputs.get("temperature")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.7);
        let top_p = node.inputs.get("top_p")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.9);
        let seed = node.inputs.get("seed")
            .and_then(|v| v.as_i64())
            .unwrap_or(-1);

        let llm_config = ctx.get_extra_data("llm_config")
            .cloned()
            .unwrap_or(json!({
                "mode": "local",
                "cli_path": "/home/acproject/workspace/rust_projects/comfyui-rust/cpp/llama.cpp-qwen3-omni/build/bin/llama-cli",
            }));

        let model_path = llm.get("model_path")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let prompt_text = prompt.as_str().unwrap_or("").to_string();

        Box::pin(async move {
            let mode = llm_config.get("mode")
                .and_then(|v| v.as_str())
                .unwrap_or("local");

            if mode == "remote" {
                let api_url = llm_config.get("api_url")
                    .and_then(|v| v.as_str())
                    .unwrap_or("http://127.0.0.1:8080");
                let api_key = llm_config.get("api_key")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");

                let client = reqwest::Client::new();
                let mut body = serde_json::json!({
                    "model": model_path,
                    "prompt": prompt_text,
                    "max_tokens": max_tokens,
                    "temperature": temperature,
                    "top_p": top_p,
                });
                if seed >= 0 {
                    body["seed"] = json!(seed);
                }
                if let Some(ref sp) = system_prompt {
                    body["system_prompt"] = json!(sp);
                }

                let mut req = client
                    .post(format!("{}/v1/completions", api_url.trim_end_matches('/')))
                    .header("Content-Type", "application/json")
                    .json(&body);

                if !api_key.is_empty() {
                    req = req.header("Authorization", format!("Bearer {}", api_key));
                }

                match req.send().await {
                    Ok(resp) => {
                        match resp.json::<serde_json::Value>().await {
                            Ok(data) => {
                                let text = data.get("choices")
                                    .and_then(|c| c.get(0))
                                    .and_then(|c| c.get("text"))
                                    .and_then(|t| t.as_str())
                                    .unwrap_or("")
                                    .to_string();
                                Ok(vec![json!(text)])
                            }
                            Err(e) => {
                                Err(ExecutorError::NodeExecutionFailed {
                                    node_id: node_id.to_string(),
                                    message: format!("Failed to parse LLM API response: {}", e),
                                })
                            }
                        }
                    }
                    Err(e) => {
                        Err(ExecutorError::NodeExecutionFailed {
                            node_id: node_id.to_string(),
                            message: format!("LLM API request failed: {}", e),
                        })
                    }
                }
            } else {
                let cli_path = llm_config.get("cli_path")
                    .and_then(|v| v.as_str())
                    .unwrap_or("/home/acproject/workspace/rust_projects/comfyui-rust/cpp/llama.cpp-qwen3-omni/build/bin/llama-cli")
                    .to_string();

                if model_path.is_empty() {
                    return Err(ExecutorError::NodeExecutionFailed {
                        node_id: node_id.to_string(),
                        message: "LLM model path is empty".to_string(),
                    });
                }

                let mut cmd = tokio::process::Command::new(&cli_path);
                cmd.arg("-m").arg(&model_path)
                    .arg("-p").arg(&prompt_text)
                    .arg("--n-predict").arg(max_tokens.to_string())
                    .arg("--temp").arg(temperature.to_string())
                    .arg("--top-p").arg(top_p.to_string())
                    .arg("--no-display-prompt")
                    .arg("--log-disable")
                    .stdout(std::process::Stdio::piped())
                    .stderr(std::process::Stdio::piped());

                if let Some(mmproj) = llm.get("mmproj_path").and_then(|v| v.as_str()) {
                    if !mmproj.is_empty() {
                        cmd.arg("--mmproj").arg(mmproj);
                        tracing::info!("LLMTextGen: auto-detected mmproj: {}", mmproj);
                    }
                }

                if seed >= 0 {
                    cmd.arg("--seed").arg(seed.to_string());
                }

                if let Some(ref sp) = system_prompt {
                    cmd.arg("--system-prompt").arg(sp);
                }

                if let Some(extra_args) = llm_config.get("extra_args").and_then(|v| v.as_str()) {
                    for arg in extra_args.split_whitespace() {
                        cmd.arg(arg);
                    }
                }

                match cmd.output().await {
                    Ok(output) => {
                        if output.status.success() {
                            let text = String::from_utf8_lossy(&output.stdout).to_string();
                            let cleaned = text.trim().to_string();
                            Ok(vec![json!(cleaned)])
                        } else {
                            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
                            Err(ExecutorError::NodeExecutionFailed {
                                node_id: node_id.to_string(),
                                message: format!("llama-cli failed: {}", stderr),
                            })
                        }
                    }
                    Err(e) => {
                        Err(ExecutorError::NodeExecutionFailed {
                            node_id: node_id.to_string(),
                            message: format!("Failed to execute llama-cli: {}", e),
                        })
                    }
                }
            }
        })
    }))
}

fn register_llm_text_gen_remote(registry: &mut NodeRegistry) {
    let class_def = NodeClassDef {
        class_type: "LLMTextGenRemote".to_string(),
        display_name: "LLM Text Generation (Remote API)".to_string(),
        category: "llm".to_string(),
        input_types: NodeInputTypes {
            required: {
                let mut m = HashMap::new();
                m.insert("prompt".to_string(), InputTypeSpec {
                    type_name: "STRING".to_string(),
                    extra: HashMap::new(),
                });
                m
            },
            optional: {
                let mut m = HashMap::new();
                m.insert("system_prompt".to_string(), InputTypeSpec {
                    type_name: "STRING".to_string(),
                    extra: HashMap::new(),
                });
                m.insert("model".to_string(), InputTypeSpec {
                    type_name: "STRING".to_string(),
                    extra: HashMap::new(),
                });
                m.insert("max_tokens".to_string(), InputTypeSpec {
                    type_name: "INT".to_string(),
                    extra: HashMap::new(),
                });
                m.insert("temperature".to_string(), InputTypeSpec {
                    type_name: "FLOAT".to_string(),
                    extra: HashMap::new(),
                });
                m.insert("top_p".to_string(), InputTypeSpec {
                    type_name: "FLOAT".to_string(),
                    extra: HashMap::new(),
                });
                m.insert("seed".to_string(), InputTypeSpec {
                    type_name: "INT".to_string(),
                    extra: HashMap::new(),
                });
                m
            },
            hidden: HashMap::new(),
        },
        output_types: vec![IoType::String],
        output_names: vec!["STRING".to_string()],
        output_is_list: vec![false],
        is_output_node: false,
        has_intermediate_output: false,
        is_changed: None,
        not_idempotent: true,
        function_name: "generate".to_string(),
    };

    registry.register(class_def, Arc::new(|ctx, node, node_id| {
        let prompt = ctx.resolve_input(node_id, "prompt")
            .unwrap_or_else(|_| json!(""));
        let system_prompt = ctx.resolve_input(node_id, "system_prompt")
            .ok()
            .and_then(|v| v.as_str().map(|s| s.to_string()));
        let model_name = node.inputs.get("model")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let max_tokens = node.inputs.get("max_tokens")
            .and_then(|v| v.as_i64())
            .unwrap_or(512);
        let temperature = node.inputs.get("temperature")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.7);
        let top_p = node.inputs.get("top_p")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.9);
        let seed = node.inputs.get("seed")
            .and_then(|v| v.as_i64())
            .unwrap_or(-1);

        let llm_config = ctx.get_extra_data("llm_config")
            .cloned()
            .unwrap_or(json!({
                "mode": "remote",
                "api_url": "http://127.0.0.1:8080",
                "api_key": "",
            }));

        let prompt_text = prompt.as_str().unwrap_or("").to_string();
        let model_str = if model_name.is_empty() {
            llm_config.get("model")
                .and_then(|v| v.as_str())
                .unwrap_or("default")
                .to_string()
        } else {
            model_name.to_string()
        };

        Box::pin(async move {
            let api_url = llm_config.get("api_url")
                .and_then(|v| v.as_str())
                .unwrap_or("http://127.0.0.1:8080");
            let api_key = llm_config.get("api_key")
                .and_then(|v| v.as_str())
                .unwrap_or("");

            let client = reqwest::Client::new();
            let mut body = serde_json::json!({
                "model": model_str,
                "prompt": prompt_text,
                "max_tokens": max_tokens,
                "temperature": temperature,
                "top_p": top_p,
            });
            if seed >= 0 {
                body["seed"] = json!(seed);
            }
            if let Some(ref sp) = system_prompt {
                body["system_prompt"] = json!(sp);
            }

            let mut req = client
                .post(format!("{}/v1/completions", api_url.trim_end_matches('/')))
                .header("Content-Type", "application/json")
                .json(&body);

            if !api_key.is_empty() {
                req = req.header("Authorization", format!("Bearer {}", api_key));
            }

            match req.send().await {
                Ok(resp) => {
                    match resp.json::<serde_json::Value>().await {
                        Ok(data) => {
                            let text = data.get("choices")
                                .and_then(|c| c.get(0))
                                .and_then(|c| c.get("text"))
                                .and_then(|t| t.as_str())
                                .unwrap_or("")
                                .to_string();
                            Ok(vec![json!(text)])
                        }
                        Err(e) => {
                            Err(ExecutorError::NodeExecutionFailed {
                                node_id: node_id.to_string(),
                                message: format!("Failed to parse remote LLM API response: {}", e),
                            })
                        }
                    }
                }
                Err(e) => {
                    Err(ExecutorError::NodeExecutionFailed {
                        node_id: node_id.to_string(),
                        message: format!("Remote LLM API request failed: {}", e),
                    })
                }
            }
        })
    }));
}

fn register_save_image(registry: &mut NodeRegistry) {
    let class_def = NodeClassDef {
        class_type: "SaveImage".to_string(),
        display_name: "Save Image".to_string(),
        category: "image".to_string(),
        input_types: NodeInputTypes {
            required: {
                let mut m = HashMap::new();
                m.insert("images".to_string(), InputTypeSpec {
                    type_name: "IMAGE".to_string(),
                    extra: HashMap::new(),
                });
                m
            },
            optional: {
                let mut m = HashMap::new();
                m.insert("filename_prefix".to_string(), InputTypeSpec {
                    type_name: "STRING".to_string(),
                    extra: HashMap::new(),
                });
                m
            },
            hidden: HashMap::new(),
        },
        output_types: vec![IoType::Image],
        output_names: vec!["IMAGES".to_string()],
        output_is_list: vec![false],
        is_output_node: true,
        has_intermediate_output: false,
        is_changed: None,
        not_idempotent: false,
        function_name: "save".to_string(),
    };

    registry.register(class_def, Arc::new(|ctx, _node, node_id| {
        let images = ctx.resolve_input(node_id, "images")
            .unwrap_or_else(|_| json!(null));
        let filename_prefix = ctx.resolve_input(node_id, "filename_prefix")
            .unwrap_or_else(|_| json!("ComfyUI"));

        let output_dir = std::env::var("COMFY_OUTPUT_DIR")
            .unwrap_or_else(|_| "output".to_string());

        Box::pin(async move {
            let prefix = filename_prefix.as_str().unwrap_or("ComfyUI");
            tracing::info!("SaveImage: saving image with prefix {}", prefix);

            let output_path = std::path::PathBuf::from(&output_dir);
            std::fs::create_dir_all(&output_path).ok();

            let image_list = images.get("images")
                .and_then(|v| v.as_array())
                .or_else(|| images.get("samples").and_then(|v| v.as_array()));

            let mut saved_images = serde_json::Value::Array(vec![]);

            if let Some(img_arr) = image_list {
                if let Some(arr) = saved_images.as_array_mut() {
                    for (i, sample) in img_arr.iter().enumerate() {
                        if let Ok(sd_image) = serde_json::from_value::<comfy_inference::SdImage>(sample.clone()) {
                            let filename = format!("{}_{:05}.png", prefix, i);
                            let filepath = output_path.join(&filename);
                            match sd_image.to_png_bytes() {
                                Ok(png_bytes) => {
                                    match std::fs::write(&filepath, &png_bytes) {
                                        Ok(_) => {
                                            arr.push(json!({
                                                "filename": filename,
                                                "subfolder": "",
                                                "type": "output"
                                            }));
                                            tracing::info!("SaveImage: saved to {}", filepath.display());
                                        }
                                        Err(e) => {
                                            tracing::error!("SaveImage: failed to write {}: {}", filepath.display(), e);
                                        }
                                    }
                                }
                                Err(e) => {
                                    tracing::error!("SaveImage: failed to encode PNG for image {}: {}", i, e);
                                }
                            }
                        }
                    }
                }
            }

            Ok(vec![json!({ "images": saved_images })])
        })
    }));
}

fn register_save_video(registry: &mut NodeRegistry) {
    let class_def = NodeClassDef {
        class_type: "SaveVideo".to_string(),
        display_name: "Save Video".to_string(),
        category: "video".to_string(),
        input_types: NodeInputTypes {
            required: {
                let mut m = HashMap::new();
                m.insert("video".to_string(), InputTypeSpec {
                    type_name: "VIDEO".to_string(),
                    extra: HashMap::new(),
                });
                m
            },
            optional: {
                let mut m = HashMap::new();
                m.insert("filename_prefix".to_string(), InputTypeSpec {
                    type_name: "STRING".to_string(),
                    extra: HashMap::new(),
                });
                m.insert("fps".to_string(), InputTypeSpec {
                    type_name: "INT".to_string(),
                    extra: HashMap::new(),
                });
                m.insert("format".to_string(), InputTypeSpec {
                    type_name: "COMBO".to_string(),
                    extra: HashMap::new(),
                });
                m
            },
            hidden: HashMap::new(),
        },
        output_types: vec![IoType::Video],
        output_names: vec!["VIDEO".to_string()],
        output_is_list: vec![false],
        is_output_node: true,
        has_intermediate_output: false,
        is_changed: None,
        not_idempotent: false,
        function_name: "save".to_string(),
    };

    registry.register(class_def, Arc::new(|ctx, node, node_id| {
        let video = ctx.resolve_input(node_id, "video")
            .unwrap_or_else(|_| json!(null));
        let filename_prefix = ctx.resolve_input(node_id, "filename_prefix")
            .unwrap_or_else(|_| json!("ComfyUI"));
        let fps = node.inputs.get("fps")
            .and_then(|v| v.as_i64())
            .unwrap_or(8);
        let format = node.inputs.get("format")
            .and_then(|v| v.as_str())
            .unwrap_or("gif");

        let output_dir = std::env::var("COMFY_OUTPUT_DIR")
            .unwrap_or_else(|_| "output".to_string());

        Box::pin(async move {
            let prefix = filename_prefix.as_str().unwrap_or("ComfyUI");
            let output_path = std::path::PathBuf::from(&output_dir);
            std::fs::create_dir_all(&output_path).ok();

            let frames = video.get("frames")
                .and_then(|v| v.as_array())
                .cloned()
                .unwrap_or_default();

            if frames.is_empty() {
                return Err(ExecutorError::NodeExecutionFailed {
                    node_id: node_id.to_string(),
                    message: "No video frames to save".to_string(),
                });
            }

            let sd_video = comfy_inference::SdVideo::new(
                frames.iter()
                    .filter_map(|f| serde_json::from_value::<comfy_inference::SdImage>(f.clone()).ok())
                    .collect(),
                fps as i32,
            );

            if sd_video.is_empty() {
                return Err(ExecutorError::NodeExecutionFailed {
                    node_id: node_id.to_string(),
                    message: "Failed to parse video frames".to_string(),
                });
            }

            let ext = match format {
                "mp4" => "mp4",
                "webm" => "webm",
                _ => "gif",
            };
            let filename = format!("{}_{}.{}", prefix, chrono::Utc::now().format("%Y%m%d_%H%M%S"), ext);
            let filepath = output_path.join(&filename);

            let encode_result = match ext {
                "mp4" => {
                    if comfy_inference::SdVideo::is_ffmpeg_available() {
                        sd_video.encode_with_ffmpeg(&filepath, fps as i32, 18)
                            .map_err(|e| e.to_string())
                    } else {
                        Err("FFmpeg is not available for MP4 encoding".to_string())
                    }
                }
                "webm" => {
                    if comfy_inference::SdVideo::is_ffmpeg_available() {
                        sd_video.encode_webm_with_ffmpeg(&filepath, fps as i32, 30)
                            .map_err(|e| e.to_string())
                    } else {
                        Err("FFmpeg is not available for WebM encoding".to_string())
                    }
                }
                _ => {
                     match sd_video.to_gif_bytes() {
                         Ok(bytes) => std::fs::write(&filepath, &bytes).map_err(|e| e.to_string()),
                         Err(e) => Err(e.to_string()),
                     }
                 }
            };

            match encode_result {
                Ok(_) => {
                    tracing::info!("SaveVideo: saved {} frames to {}", sd_video.frame_count(), filepath.display());
                    Ok(vec![json!({
                        "type": "video",
                        "videos": [{
                            "filename": filename,
                            "subfolder": "",
                            "type": "output",
                            "frame_count": sd_video.frame_count(),
                            "fps": fps,
                        }]
                    })])
                }
                Err(e) => {
                    Err(ExecutorError::NodeExecutionFailed {
                        node_id: node_id.to_string(),
                        message: format!("Failed to encode video: {}", e),
                    })
                }
            }
        })
    }));
}

fn register_load_video(registry: &mut NodeRegistry) {
    let video_choices = scan_input_videos();

    let class_def = NodeClassDef {
        class_type: "LoadVideo".to_string(),
        display_name: "Load Video".to_string(),
        category: "video".to_string(),
        input_types: NodeInputTypes {
            required: {
                let mut m = HashMap::new();
                m.insert("video".to_string(), InputTypeSpec {
                    type_name: "COMBO".to_string(),
                    extra: {
                        let mut e = HashMap::new();
                        e.insert("choices".to_string(), serde_json::Value::Array(
                            video_choices.iter().map(|s| json!(s)).collect()
                        ));
                        e
                    },
                });
                m
            },
            optional: {
                let mut m = HashMap::new();
                m.insert("fps".to_string(), InputTypeSpec {
                    type_name: "INT".to_string(),
                    extra: HashMap::new(),
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
        function_name: "load".to_string(),
    };

    registry.register(class_def, Arc::new(|_ctx, node, _node_id| {
        let video_path = node.inputs.get("video")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let fps = node.inputs.get("fps")
            .and_then(|v| v.as_i64())
            .unwrap_or(8);

        let video_path = video_path.to_string();

        Box::pin(async move {
            if video_path.is_empty() {
                return Err(ExecutorError::NodeExecutionFailed {
                    node_id: _node_id.to_string(),
                    message: "No video file specified".to_string(),
                });
            }

            let path = std::path::PathBuf::from(&video_path);
            if !path.exists() {
                return Err(ExecutorError::NodeExecutionFailed {
                    node_id: _node_id.to_string(),
                    message: format!("Video file not found: {}", video_path),
                });
            }

            let ext = path.extension()
                .and_then(|e| e.to_str())
                .unwrap_or("")
                .to_lowercase();

            if ext == "gif" {
                let data = std::fs::read(&path).map_err(|e| ExecutorError::NodeExecutionFailed {
                    node_id: _node_id.to_string(),
                    message: format!("Failed to read video file: {}", e),
                })?;

                let mut decoder = gif::DecodeOptions::new();
                decoder.set_color_output(gif::ColorOutput::RGBA);
                let mut reader = decoder.read_info(std::io::Cursor::new(data))
                    .map_err(|e| ExecutorError::NodeExecutionFailed {
                        node_id: _node_id.to_string(),
                        message: format!("Failed to decode GIF: {}", e),
                    })?;

                let mut frames = Vec::new();
                while let Ok(Some(frame)) = reader.read_next_frame() {
                    let w = frame.width as u32;
                    let h = frame.height as u32;
                    let buf = frame.buffer.to_vec();
                    if let Ok(img) = comfy_inference::SdImage::rgba(w, h, buf) {
                        frames.push(img);
                    }
                }

                let video = comfy_inference::SdVideo::new(frames, fps as i32);
                let val = serde_json::to_value(&video).map_err(|e| ExecutorError::NodeExecutionFailed {
                    node_id: _node_id.to_string(),
                    message: format!("Failed to serialize video: {}", e),
                })?;

                Ok(vec![json!({
                    "type": "video",
                    "videos": [{
                        "filename": path.file_name().and_then(|n| n.to_str()).unwrap_or("video.gif"),
                        "subfolder": "",
                        "type": "input",
                    }],
                    "frames": val.get("frames").cloned().unwrap_or(json!([])),
                    "fps": fps,
                })])
            } else if ["mp4", "webm", "avi", "mov"].contains(&ext.as_str()) {
                if !comfy_inference::SdVideo::is_ffmpeg_available() {
                    return Err(ExecutorError::NodeExecutionFailed {
                        node_id: _node_id.to_string(),
                        message: format!("FFmpeg is required to decode {} files but is not available", ext),
                    });
                }

                let video = comfy_inference::SdVideo::decode_with_ffmpeg(&path, fps as i32)
                    .map_err(|e| ExecutorError::NodeExecutionFailed {
                        node_id: _node_id.to_string(),
                        message: format!("Failed to decode video with FFmpeg: {}", e),
                    })?;

                let frame_count = video.frame_count();
                let val = serde_json::to_value(&video).map_err(|e| ExecutorError::NodeExecutionFailed {
                    node_id: _node_id.to_string(),
                    message: format!("Failed to serialize video: {}", e),
                })?;

                tracing::info!("LoadVideo: decoded {} frames from {}", frame_count, video_path);

                Ok(vec![json!({
                    "type": "video",
                    "videos": [{
                        "filename": path.file_name().and_then(|n| n.to_str()).unwrap_or(&video_path),
                        "subfolder": "",
                        "type": "input",
                    }],
                    "frames": val.get("frames").cloned().unwrap_or(json!([])),
                    "fps": fps,
                })])
            } else {
                Err(ExecutorError::NodeExecutionFailed {
                    node_id: _node_id.to_string(),
                    message: format!("Unsupported video format: {}. Supported: gif, mp4, webm, avi, mov", ext),
                })
            }
        })
    }))
}

fn register_empty_latent_image(registry: &mut NodeRegistry) {
    let class_def = NodeClassDef {
        class_type: "EmptyLatentImage".to_string(),
        display_name: "Empty Latent Image".to_string(),
        category: "latent".to_string(),
        input_types: NodeInputTypes {
            required: {
                let mut m = HashMap::new();
                m.insert("width".to_string(), InputTypeSpec {
                    type_name: "INT".to_string(),
                    extra: HashMap::new(),
                });
                m.insert("height".to_string(), InputTypeSpec {
                    type_name: "INT".to_string(),
                    extra: HashMap::new(),
                });
                m.insert("batch_size".to_string(), InputTypeSpec {
                    type_name: "INT".to_string(),
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
        function_name: "generate".to_string(),
    };

    registry.register(class_def, Arc::new(|_ctx, node, _node_id| {
        let width = node.inputs.get("width")
            .and_then(|v| v.as_i64())
            .unwrap_or(512);
        let height = node.inputs.get("height")
            .and_then(|v| v.as_i64())
            .unwrap_or(512);
        let batch_size = node.inputs.get("batch_size")
            .and_then(|v| v.as_i64())
            .unwrap_or(1);

        Box::pin(async move {
            Ok(vec![json!({
                "type": "latent",
                "width": width,
                "height": height,
                "batch_size": batch_size,
            })])
        })
    }));
}

fn register_vae_decode(registry: &mut NodeRegistry) {
    let class_def = NodeClassDef {
        class_type: "VAEDecode".to_string(),
        display_name: "VAE Decode".to_string(),
        category: "latent".to_string(),
        input_types: NodeInputTypes {
            required: {
                let mut m = HashMap::new();
                m.insert("samples".to_string(), InputTypeSpec {
                    type_name: "LATENT".to_string(),
                    extra: HashMap::new(),
                });
                m.insert("vae".to_string(), InputTypeSpec {
                    type_name: "VAE".to_string(),
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
        function_name: "decode".to_string(),
    };

    registry.register(class_def, Arc::new(|ctx, _node, node_id| {
        let samples = ctx.resolve_input(node_id, "samples")
            .unwrap_or_else(|_| json!(null));
        let _vae = ctx.resolve_input(node_id, "vae")
            .unwrap_or_else(|_| json!(null));

        Box::pin(async move {
            if let Some(sample_arr) = samples.get("samples").and_then(|v| v.as_array()) {
                if !sample_arr.is_empty() {
                    return Ok(vec![json!({
                        "type": "image",
                        "images": sample_arr,
                    })]);
                }
            }

            Ok(vec![json!({
                "type": "image",
                "source": "vae_decode",
                "latent": samples,
            })])
        })
    }));
}

fn register_vae_encode(registry: &mut NodeRegistry) {
    let class_def = NodeClassDef {
        class_type: "VAEEncode".to_string(),
        display_name: "VAE Encode".to_string(),
        category: "latent".to_string(),
        input_types: NodeInputTypes {
            required: {
                let mut m = HashMap::new();
                m.insert("pixels".to_string(), InputTypeSpec {
                    type_name: "IMAGE".to_string(),
                    extra: HashMap::new(),
                });
                m.insert("vae".to_string(), InputTypeSpec {
                    type_name: "VAE".to_string(),
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
        function_name: "encode".to_string(),
    };

    registry.register(class_def, Arc::new(|ctx, _node, node_id| {
        let pixels = ctx.resolve_input(node_id, "pixels")
            .unwrap_or_else(|_| json!(null));
        let vae = ctx.resolve_input(node_id, "vae")
            .unwrap_or_else(|_| json!(null));

        Box::pin(async move {
            if let Some(images) = pixels.get("images").and_then(|v| v.as_array()) {
                if !images.is_empty() {
                    return Ok(vec![json!({
                        "type": "latent",
                        "samples": images,
                    })]);
                }
            }

            let width = pixels.get("width").and_then(|v| v.as_i64()).unwrap_or(512);
            let height = pixels.get("height").and_then(|v| v.as_i64()).unwrap_or(512);

            Ok(vec![json!({
                "type": "latent",
                "source": "vae_encode",
                "image": pixels,
                "vae": vae,
                "width": width,
                "height": height,
            })])
        })
    }));
}

fn register_load_image(registry: &mut NodeRegistry) {
    let image_choices = scan_input_images();

    let class_def = NodeClassDef {
        class_type: "LoadImage".to_string(),
        display_name: "Load Image".to_string(),
        category: "image".to_string(),
        input_types: NodeInputTypes {
            required: {
                let mut m = HashMap::new();
                m.insert("image".to_string(), InputTypeSpec {
                    type_name: "COMBO".to_string(),
                    extra: {
                        let mut e = HashMap::new();
                        e.insert("choices".to_string(), serde_json::Value::Array(
                            image_choices.iter().map(|s| json!(s)).collect()
                        ));
                        e
                    },
                });
                m
            },
            optional: HashMap::new(),
            hidden: HashMap::new(),
        },
        output_types: vec![IoType::Image, IoType::Mask],
        output_names: vec!["IMAGE".to_string(), "MASK".to_string()],
        output_is_list: vec![false, false],
        is_output_node: false,
        has_intermediate_output: false,
        is_changed: None,
        not_idempotent: false,
        function_name: "load".to_string(),
    };

    registry.register(class_def, Arc::new(|_ctx, node, _node_id| {
        let image_path = node.inputs.get("image")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        Box::pin(async move {
            Ok(vec![
                json!({
                    "type": "image",
                    "path": image_path,
                }),
                json!({
                    "type": "mask",
                    "path": image_path,
                }),
            ])
        })
    }));
}

fn register_upscale_image(registry: &mut NodeRegistry) {
    let class_def = NodeClassDef {
        class_type: "ImageUpscaleWithModel".to_string(),
        display_name: "Upscale Image (using Model)".to_string(),
        category: "image/upscaling".to_string(),
        input_types: NodeInputTypes {
            required: {
                let mut m = HashMap::new();
                m.insert("upscale_model".to_string(), InputTypeSpec {
                    type_name: "UPSCALE_MODEL".to_string(),
                    extra: HashMap::new(),
                });
                m.insert("image".to_string(), InputTypeSpec {
                    type_name: "IMAGE".to_string(),
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
        function_name: "upscale".to_string(),
    };

    registry.register(class_def, Arc::new(|ctx, _node, node_id| {
        let upscale_model = ctx.resolve_input(node_id, "upscale_model")
            .unwrap_or_else(|_| json!(null));
        let image = ctx.resolve_input(node_id, "image")
            .unwrap_or_else(|_| json!(null));

        let backend = ctx.backend();

        Box::pin(async move {
            let esrgan_path = upscale_model.get("path")
                .and_then(|v| v.as_str())
                .unwrap_or("");

            if esrgan_path.is_empty() {
                return Ok(vec![image.clone()]);
            }

            let params = comfy_inference::UpscaleParams::new(esrgan_path);

            let placeholder = comfy_inference::SdImage::new(1, 1, 3);
            match backend.upscale(placeholder, params) {
                Ok(_result) => {
                    Ok(vec![json!({
                        "type": "image",
                        "source": "upscale",
                        "upscale_model": upscale_model,
                        "input_image": image,
                    })])
                }
                Err(e) => {
                    tracing::warn!("Upscale failed: {}, returning original image", e);
                    Ok(vec![image.clone()])
                }
            }
        })
    }));
}

fn register_clip_vision_encode(registry: &mut NodeRegistry) {
    let class_def = NodeClassDef {
        class_type: "CLIPVisionEncode".to_string(),
        display_name: "CLIP Vision Encode".to_string(),
        category: "conditioning".to_string(),
        input_types: NodeInputTypes {
            required: {
                let mut m = HashMap::new();
                m.insert("clip_vision".to_string(), InputTypeSpec {
                    type_name: "CLIP_VISION".to_string(),
                    extra: HashMap::new(),
                });
                m.insert("image".to_string(), InputTypeSpec {
                    type_name: "IMAGE".to_string(),
                    extra: HashMap::new(),
                });
                m
            },
            optional: HashMap::new(),
            hidden: HashMap::new(),
        },
        output_types: vec![IoType::Conditioning],
        output_names: vec!["CONDITIONING".to_string()],
        output_is_list: vec![false],
        is_output_node: false,
        has_intermediate_output: false,
        is_changed: None,
        not_idempotent: false,
        function_name: "encode".to_string(),
    };

    registry.register(class_def, Arc::new(|ctx, _node, node_id| {
        let clip_vision = ctx.resolve_input(node_id, "clip_vision")
            .unwrap_or_else(|_| json!(null));
        let image = ctx.resolve_input(node_id, "image")
            .unwrap_or_else(|_| json!(null));

        Box::pin(async move {
            Ok(vec![json!({
                "type": "conditioning",
                "clip_vision": clip_vision,
                "image": image,
            })])
        })
    }));
}

fn register_control_net_apply(registry: &mut NodeRegistry) {
    let class_def = NodeClassDef {
        class_type: "ControlNetApply".to_string(),
        display_name: "Apply ControlNet".to_string(),
        category: "conditioning".to_string(),
        input_types: NodeInputTypes {
            required: {
                let mut m = HashMap::new();
                m.insert("conditioning".to_string(), InputTypeSpec {
                    type_name: "CONDITIONING".to_string(),
                    extra: HashMap::new(),
                });
                m.insert("control_net".to_string(), InputTypeSpec {
                    type_name: "CONTROL_NET".to_string(),
                    extra: HashMap::new(),
                });
                m.insert("image".to_string(), InputTypeSpec {
                    type_name: "IMAGE".to_string(),
                    extra: HashMap::new(),
                });
                m.insert("strength".to_string(), InputTypeSpec {
                    type_name: "FLOAT".to_string(),
                    extra: HashMap::new(),
                });
                m
            },
            optional: HashMap::new(),
            hidden: HashMap::new(),
        },
        output_types: vec![IoType::Conditioning],
        output_names: vec!["CONDITIONING".to_string()],
        output_is_list: vec![false],
        is_output_node: false,
        has_intermediate_output: false,
        is_changed: None,
        not_idempotent: false,
        function_name: "apply".to_string(),
    };

    registry.register(class_def, Arc::new(|ctx, _node, node_id| {
        let conditioning = ctx.resolve_input(node_id, "conditioning")
            .unwrap_or_else(|_| json!(null));
        let control_net = ctx.resolve_input(node_id, "control_net")
            .unwrap_or_else(|_| json!(null));
        let image = ctx.resolve_input(node_id, "image")
            .unwrap_or_else(|_| json!(null));
        let strength = ctx.resolve_input(node_id, "strength")
            .unwrap_or_else(|_| json!(1.0));

        Box::pin(async move {
            let mut result = conditioning.as_object().cloned().unwrap_or_default();
            result.insert("control_net".to_string(), control_net);
            result.insert("control_image".to_string(), image);
            result.insert("control_strength".to_string(), strength);
            Ok(vec![json!(result)])
        })
    }));
}

fn register_convert_model(registry: &mut NodeRegistry) {
    let class_def = NodeClassDef {
        class_type: "ConvertModel".to_string(),
        display_name: "Convert Model".to_string(),
        category: "model_management".to_string(),
        input_types: NodeInputTypes {
            required: {
                let mut m = HashMap::new();
                m.insert("input_path".to_string(), InputTypeSpec {
                    type_name: "STRING".to_string(),
                    extra: HashMap::new(),
                });
                m.insert("output_path".to_string(), InputTypeSpec {
                    type_name: "STRING".to_string(),
                    extra: HashMap::new(),
                });
                m.insert("output_type".to_string(), InputTypeSpec {
                    type_name: "COMBO".to_string(),
                    extra: HashMap::new(),
                });
                m
            },
            optional: {
                let mut m = HashMap::new();
                m.insert("vae_path".to_string(), InputTypeSpec {
                    type_name: "STRING".to_string(),
                    extra: HashMap::new(),
                });
                m.insert("tensor_type_rules".to_string(), InputTypeSpec {
                    type_name: "STRING".to_string(),
                    extra: HashMap::new(),
                });
                m
            },
            hidden: HashMap::new(),
        },
        output_types: vec![IoType::String],
        output_names: vec!["OUTPUT_PATH".to_string()],
        output_is_list: vec![false],
        is_output_node: true,
        has_intermediate_output: false,
        is_changed: None,
        not_idempotent: false,
        function_name: "convert".to_string(),
    };

    registry.register(class_def, Arc::new(|_ctx, node, _node_id| {
        let input_path = node.inputs.get("input_path")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let output_path = node.inputs.get("output_path")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let output_type_str = node.inputs.get("output_type")
            .and_then(|v| v.as_str())
            .unwrap_or("q8_0");
        let vae_path = node.inputs.get("vae_path")
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty());
        let tensor_type_rules = node.inputs.get("tensor_type_rules")
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty());

        let input_path = input_path.to_string();
        let output_path = output_path.to_string();
        let output_type_str = output_type_str.to_string();
        let vae_path = vae_path.map(|s| s.to_string());
        let tensor_type_rules = tensor_type_rules.map(|s| s.to_string());

        Box::pin(async move {
            #[cfg(feature = "local-ffi")]
            {
                let sd_type = parse_sd_type(&output_type_str);
                let mut params = ConvertParams::new(&input_path, &output_path)
                    .with_output_type(sd_type);
                if let Some(ref vae) = vae_path {
                    params = params.with_vae(vae);
                }
                if let Some(ref rules) = tensor_type_rules {
                    params = params.with_tensor_type_rules(rules);
                }

                match convert_model(params) {
                    Ok(true) => Ok(vec![json!(output_path)]),
                    Ok(false) => Err(ExecutorError::NodeExecutionFailed {
                        node_id: String::new(),
                        message: "Model conversion returned false".to_string(),
                    }),
                    Err(e) => Err(ExecutorError::Inference(e)),
                }
            }
            #[cfg(not(feature = "local-ffi"))]
            {
                let _ = (input_path, output_path, output_type_str, vae_path, tensor_type_rules);
                Err(ExecutorError::NodeExecutionFailed {
                    node_id: String::new(),
                    message: "Model conversion requires local-ffi feature".to_string(),
                })
            }
        })
    }));
}

#[cfg(feature = "local-ffi")]
fn parse_sd_type(name: &str) -> SdType {
    match name {
        "f32" => SdType::F32,
        "f16" => SdType::F16,
        "bf16" => SdType::BF16,
        "q4_0" => SdType::Q4_0,
        "q4_1" => SdType::Q4_1,
        "q5_0" => SdType::Q5_0,
        "q5_1" => SdType::Q5_1,
        "q8_0" => SdType::Q8_0,
        "q8_1" => SdType::Q8_1,
        "q2_k" => SdType::Q2_K,
        "q3_k" => SdType::Q3_K,
        "q4_k" => SdType::Q4_K,
        "q5_k" => SdType::Q5_K,
        "q6_k" => SdType::Q6_K,
        "q8_k" => SdType::Q8_K,
        _ => SdType::Q8_0,
    }
}

fn parse_sample_method(name: &str) -> SampleMethod {
    match name {
        "euler" => SampleMethod::Euler,
        "euler_ancestral" | "euler_a" => SampleMethod::EulerA,
        "heun" => SampleMethod::Heun,
        "dpm_2" => SampleMethod::DPM2,
        "dpmpp_2s_ancestral" => SampleMethod::DPMPP2SA,
        "dpmpp_2m" => SampleMethod::DPMPP2M,
        "dpmpp_2m_sde" | "dpmpp_2m_v2" => SampleMethod::DPMPP2Mv2,
        "ipndm" => SampleMethod::IPNDM,
        "ipndm_v" => SampleMethod::IPNDMV,
        "lcm" => SampleMethod::LCM,
        "ddim" => SampleMethod::DDIMTrailing,
        "tcd" => SampleMethod::TCD,
        "res_multistep" => SampleMethod::ResMultistep,
        "res_2s" => SampleMethod::Res2S,
        "er_sde" => SampleMethod::ErSde,
        _ => SampleMethod::EulerA,
    }
}

fn parse_scheduler(name: &str) -> Scheduler {
    match name {
        "normal" | "discrete" => Scheduler::Discrete,
        "karras" => Scheduler::Karras,
        "exponential" => Scheduler::Exponential,
        "ays" | "sgm_uniform" => Scheduler::SgmUniform,
        "simple" => Scheduler::Simple,
        "smoothstep" => Scheduler::Smoothstep,
        "kl_optimal" => Scheduler::KlOptimal,
        "lcm" => Scheduler::Lcm,
        "bong_tangent" => Scheduler::BongTangent,
        _ => Scheduler::Discrete,
    }
}

fn scan_input_images() -> Vec<String> {
    let input_dir = std::path::Path::new("input");
    if !input_dir.exists() {
        return Vec::new();
    }
    let mut results = Vec::new();
    scan_image_dir(input_dir, input_dir, &mut results);
    results.sort();
    results
}

fn scan_image_dir(dir: &std::path::Path, base: &std::path::Path, results: &mut Vec<String>) {
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                scan_image_dir(&path, base, results);
            } else if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                let lower = name.to_lowercase();
                if lower.ends_with(".png") || lower.ends_with(".jpg") || lower.ends_with(".jpeg")
                    || lower.ends_with(".webp") || lower.ends_with(".gif") || lower.ends_with(".bmp")
                {
                    if let Ok(rel) = path.strip_prefix(base) {
                        results.push(rel.to_string_lossy().to_string());
                    }
                }
            }
        }
    }
}

fn scan_input_videos() -> Vec<String> {
    let input_dir = std::path::Path::new("input");
    if !input_dir.exists() {
        return Vec::new();
    }
    let mut results = Vec::new();
    scan_video_dir(input_dir, input_dir, &mut results);
    results.sort();
    results
}

fn scan_video_dir(dir: &std::path::Path, base: &std::path::Path, results: &mut Vec<String>) {
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                scan_video_dir(&path, base, results);
            } else if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                let lower = name.to_lowercase();
                if lower.ends_with(".gif") || lower.ends_with(".mp4") || lower.ends_with(".webm")
                    || lower.ends_with(".avi") || lower.ends_with(".mov")
                {
                    if let Ok(rel) = path.strip_prefix(base) {
                        results.push(rel.to_string_lossy().to_string());
                    }
                }
            }
        }
    }
}

fn register_clip_loader(registry: &mut NodeRegistry) {
    let class_def = NodeClassDef {
        class_type: "CLIPLoader".to_string(),
        display_name: "Load CLIP".to_string(),
        category: "loaders".to_string(),
        input_types: NodeInputTypes {
            required: {
                let mut m = HashMap::new();
                m.insert("clip_name".to_string(), InputTypeSpec {
                    type_name: "COMBO".to_string(),
                    extra: HashMap::new(),
                });
                m.insert("type".to_string(), InputTypeSpec {
                    type_name: "COMBO".to_string(),
                    extra: HashMap::new(),
                });
                m
            },
            optional: HashMap::new(),
            hidden: HashMap::new(),
        },
        output_types: vec![IoType::Clip],
        output_names: vec!["CLIP".to_string()],
        output_is_list: vec![false],
        is_output_node: false,
        has_intermediate_output: false,
        is_changed: None,
        not_idempotent: false,
        function_name: "load_clip".to_string(),
    };

    registry.register(class_def, Arc::new(|_ctx, node, _node_id| {
        let clip_name = node.inputs.get("clip_name")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let clip_type_raw = node.inputs.get("type")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let clip_type = if clip_type_raw.is_empty() {
            let lower = clip_name.to_lowercase();
            if lower.contains("t5") {
                "sd3"
            } else if lower.contains("clip_g") {
                "stable_cascade"
            } else {
                "stable_diffusion"
            }
        } else {
            clip_type_raw
        };

        let clip_path = resolve_model_path("text_encoders", clip_name);

        let clip_type_str = clip_type.to_string();

        Box::pin(async move {
            let mut clip_config = serde_json::Map::new();
            clip_config.insert("type".to_string(), json!("clip"));
            clip_config.insert("clip_type".to_string(), json!(clip_type_str));

            match clip_type_str.as_str() {
                "stable_diffusion" => {
                    clip_config.insert("clip_l_path".to_string(), json!(clip_path));
                }
                "stable_cascade" => {
                    clip_config.insert("clip_g_path".to_string(), json!(clip_path));
                }
                "sd3" | "flux" | "wan" => {
                    clip_config.insert("t5xxl_path".to_string(), json!(clip_path));
                }
                _ => {
                    tracing::warn!(
                        "CLIPLoader: unknown type '{}', inferring from filename '{}'",
                        clip_type_str, clip_name
                    );
                    let lower = clip_name.to_lowercase();
                    if lower.contains("t5") {
                        clip_config.insert("t5xxl_path".to_string(), json!(clip_path));
                    } else if lower.contains("clip_g") {
                        clip_config.insert("clip_g_path".to_string(), json!(clip_path));
                    } else {
                        clip_config.insert("clip_l_path".to_string(), json!(clip_path));
                    }
                }
            }

            tracing::info!(
                "CLIPLoader: loaded '{}' with type '{}' -> clip_l={:?}, clip_g={:?}, t5xxl={:?}",
                clip_name, clip_type_str,
                clip_config.get("clip_l_path"),
                clip_config.get("clip_g_path"),
                clip_config.get("t5xxl_path")
            );

            Ok(vec![json!(clip_config)])
        })
    }));
}

fn register_dual_clip_loader(registry: &mut NodeRegistry) {
    let class_def = NodeClassDef {
        class_type: "DualCLIPLoader".to_string(),
        display_name: "DualCLIPLoader".to_string(),
        category: "loaders".to_string(),
        input_types: NodeInputTypes {
            required: {
                let mut m = HashMap::new();
                m.insert("clip_name1".to_string(), InputTypeSpec {
                    type_name: "COMBO".to_string(),
                    extra: HashMap::new(),
                });
                m.insert("clip_name2".to_string(), InputTypeSpec {
                    type_name: "COMBO".to_string(),
                    extra: HashMap::new(),
                });
                m.insert("type".to_string(), InputTypeSpec {
                    type_name: "COMBO".to_string(),
                    extra: HashMap::new(),
                });
                m
            },
            optional: HashMap::new(),
            hidden: HashMap::new(),
        },
        output_types: vec![IoType::Clip],
        output_names: vec!["CLIP".to_string()],
        output_is_list: vec![false],
        is_output_node: false,
        has_intermediate_output: false,
        is_changed: None,
        not_idempotent: false,
        function_name: "load_dual_clip".to_string(),
    };

    registry.register(class_def, Arc::new(|_ctx, node, _node_id| {
        let clip_name1 = node.inputs.get("clip_name1")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let clip_name2 = node.inputs.get("clip_name2")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let clip_type = node.inputs.get("type")
            .and_then(|v| v.as_str())
            .unwrap_or("sdxl");

        let clip1_path = resolve_model_path("text_encoders", clip_name1);
        let clip2_path = resolve_model_path("text_encoders", clip_name2);

        let clip_type_str = clip_type.to_string();

        Box::pin(async move {
            let mut clip_config = serde_json::Map::new();
            clip_config.insert("type".to_string(), json!("clip"));
            clip_config.insert("clip_type".to_string(), json!(clip_type_str));

            match clip_type_str.as_str() {
                "sdxl" => {
                    clip_config.insert("clip_l_path".to_string(), json!(clip1_path));
                    clip_config.insert("clip_g_path".to_string(), json!(clip2_path));
                }
                "flux" => {
                    clip_config.insert("clip_l_path".to_string(), json!(clip1_path));
                    clip_config.insert("t5xxl_path".to_string(), json!(clip2_path));
                }
                "sd3" => {
                    clip_config.insert("clip_l_path".to_string(), json!(clip1_path));
                    clip_config.insert("clip_g_path".to_string(), json!(clip1_path));
                    clip_config.insert("t5xxl_path".to_string(), json!(clip2_path));
                }
                "wan" => {
                    clip_config.insert("t5xxl_path".to_string(), json!(clip2_path));
                }
                _ => {
                    clip_config.insert("clip_l_path".to_string(), json!(clip1_path));
                    clip_config.insert("clip_g_path".to_string(), json!(clip2_path));
                }
            }

            Ok(vec![json!(clip_config)])
        })
    }));
}

fn register_wan_video_sampler(registry: &mut NodeRegistry) {
    let class_def = NodeClassDef {
        class_type: "WanVideoSampler".to_string(),
        display_name: "Wan Video Sampler".to_string(),
        category: "sampling/video".to_string(),
        input_types: NodeInputTypes {
            required: {
                let mut m = HashMap::new();
                m.insert("model".to_string(), InputTypeSpec {
                    type_name: "MODEL".to_string(),
                    extra: HashMap::new(),
                });
                m.insert("seed".to_string(), InputTypeSpec {
                    type_name: "INT".to_string(),
                    extra: HashMap::new(),
                });
                m.insert("steps".to_string(), InputTypeSpec {
                    type_name: "INT".to_string(),
                    extra: HashMap::new(),
                });
                m.insert("cfg".to_string(), InputTypeSpec {
                    type_name: "FLOAT".to_string(),
                    extra: HashMap::new(),
                });
                m.insert("sampler_name".to_string(), InputTypeSpec {
                    type_name: "COMBO".to_string(),
                    extra: HashMap::new(),
                });
                m.insert("scheduler".to_string(), InputTypeSpec {
                    type_name: "COMBO".to_string(),
                    extra: HashMap::new(),
                });
                m.insert("positive".to_string(), InputTypeSpec {
                    type_name: "CONDITIONING".to_string(),
                    extra: HashMap::new(),
                });
                m.insert("negative".to_string(), InputTypeSpec {
                    type_name: "CONDITIONING".to_string(),
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
                m.insert("video_frames".to_string(), InputTypeSpec {
                    type_name: "INT".to_string(),
                    extra: HashMap::new(),
                });
                m
            },
            optional: {
                let mut m = HashMap::new();
                m.insert("init_image".to_string(), InputTypeSpec {
                    type_name: "IMAGE".to_string(),
                    extra: HashMap::new(),
                });
                m.insert("end_image".to_string(), InputTypeSpec {
                    type_name: "IMAGE".to_string(),
                    extra: HashMap::new(),
                });
                m.insert("flow_shift".to_string(), InputTypeSpec {
                    type_name: "FLOAT".to_string(),
                    extra: HashMap::new(),
                });
                m
            },
            hidden: HashMap::new(),
        },
        output_types: vec![IoType::Latent],
        output_names: vec!["LATENT".to_string()],
        output_is_list: vec![false],
        is_output_node: false,
        has_intermediate_output: false,
        is_changed: None,
        not_idempotent: true,
        function_name: "sample_video".to_string(),
    };

    registry.register(class_def, Arc::new(|ctx, _node, node_id| {
        let model = ctx.resolve_input(node_id, "model").unwrap_or_else(|_| json!({}));
        let seed = ctx.resolve_input(node_id, "seed").unwrap_or_else(|_| json!(42));
        let steps = ctx.resolve_input(node_id, "steps").unwrap_or_else(|_| json!(20));
        let cfg = ctx.resolve_input(node_id, "cfg").unwrap_or_else(|_| json!(6.0));
        let sampler_name = ctx.resolve_input(node_id, "sampler_name")
            .unwrap_or_else(|_| json!("euler"));
        let scheduler = ctx.resolve_input(node_id, "scheduler")
            .unwrap_or_else(|_| json!("discrete"));
        let positive = ctx.resolve_input(node_id, "positive").unwrap_or_else(|_| json!(null));
        let negative = ctx.resolve_input(node_id, "negative").unwrap_or_else(|_| json!(null));
        let width = ctx.resolve_input(node_id, "width").unwrap_or_else(|_| json!(832));
        let height = ctx.resolve_input(node_id, "height").unwrap_or_else(|_| json!(480));
        let video_frames = ctx.resolve_input(node_id, "video_frames").unwrap_or_else(|_| json!(33));
        let _init_image = ctx.resolve_input(node_id, "init_image").ok();
        let _end_image = ctx.resolve_input(node_id, "end_image").ok();
        let flow_shift = ctx.resolve_input(node_id, "flow_shift")
            .ok()
            .and_then(|v| v.as_f64());

        let backend = ctx.backend();
        let supports_vid_gen = backend.supports_video_generation();

        Box::pin(async move {
            if !supports_vid_gen {
                tracing::warn!(
                    "WanVideoSampler: backend does not support video generation, skipping inference. \
                     Check that sd-cli or local inference backend is properly configured."
                );
            }
            if supports_vid_gen {
                let prompt_text = positive.get("text")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let negative_text = negative.get("text")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");

                let mut model_config = ModelConfig::default();
                if let Some(path) = model.get("model_path").and_then(|v| v.as_str()) {
                    model_config = model_config.with_model(path);
                }
                if let Some(path) = model.get("diffusion_model_path").and_then(|v| v.as_str()) {
                    model_config = model_config.with_diffusion_model(path);
                }
                if let Some(path) = model.get("vae_path").and_then(|v| v.as_str()) {
                    model_config = model_config.with_vae(path);
                }
                if let Some(path) = model.get("clip_l_path").and_then(|v| v.as_str()) {
                    model_config = model_config.with_clip_l(path);
                }
                if let Some(path) = model.get("clip_g_path").and_then(|v| v.as_str()) {
                    model_config = model_config.with_clip_g(path);
                }
                if let Some(path) = model.get("clip_vision_path").and_then(|v| v.as_str()) {
                    model_config = model_config.with_clip_vision(path);
                }
                if let Some(path) = model.get("t5xxl_path").and_then(|v| v.as_str()) {
                    model_config = model_config.with_t5xxl(path);
                }

                let clip_config = positive.get("clip");
                if let Some(clip) = clip_config {
                    if model_config.clip_l_path.is_none() {
                        if let Some(path) = clip.get("clip_l_path").and_then(|v| v.as_str()) {
                            model_config = model_config.with_clip_l(path);
                        }
                    }
                    if model_config.clip_g_path.is_none() {
                        if let Some(path) = clip.get("clip_g_path").and_then(|v| v.as_str()) {
                            model_config = model_config.with_clip_g(path);
                        }
                    }
                    if model_config.t5xxl_path.is_none() {
                        if let Some(path) = clip.get("t5xxl_path").and_then(|v| v.as_str()) {
                            model_config = model_config.with_t5xxl(path);
                        }
                    }
                }

                let needs_clip_auto_detect = model_config.clip_l_path.is_none()
                    || model_config.clip_g_path.is_none()
                    || model_config.t5xxl_path.is_none();
                let needs_vae_auto_detect = model_config.vae_path.is_none();
                if needs_clip_auto_detect || needs_vae_auto_detect {
                    let model_type_str = model.get("model_type")
                        .and_then(|v| v.as_str())
                        .unwrap_or("wan");
                    let detected_type = match model_type_str {
                        "sd3" => ModelType::SD3,
                        "flux" => ModelType::Flux,
                        "sdxl" => ModelType::SDXL,
                        "sd15" => ModelType::SD15,
                        "wan" => ModelType::Wan,
                        _ => ModelType::Wan,
                    };

                    if needs_clip_auto_detect {
                        let (clip_l, clip_g, t5xxl) = auto_detect_text_encoders(detected_type);
                        if model_config.clip_l_path.is_none() {
                            if let Some(path) = clip_l {
                                model_config = model_config.with_clip_l(path);
                            }
                        }
                        if model_config.clip_g_path.is_none() {
                            if let Some(path) = clip_g {
                                model_config = model_config.with_clip_g(path);
                            }
                        }
                        if model_config.t5xxl_path.is_none() {
                            if let Some(path) = t5xxl {
                                model_config = model_config.with_t5xxl(path);
                            }
                        }
                    }

                    if needs_vae_auto_detect {
                        if let Some(path) = auto_detect_vae(detected_type) {
                            model_config = model_config.with_vae(path);
                        }
                    }
                }

                let sample_method = parse_sample_method(
                    sampler_name.as_str().unwrap_or("euler")
                );
                let scheduler_type = parse_scheduler(
                    scheduler.as_str().unwrap_or("discrete")
                );

                let mut video_params = comfy_inference::VideoGenParams::new(prompt_text)
                    .with_negative_prompt(negative_text)
                    .with_dimensions(
                        width.as_i64().unwrap_or(832) as i32,
                        height.as_i64().unwrap_or(480) as i32,
                    )
                    .with_seed(seed.as_i64().unwrap_or(42))
                    .with_video_frames(video_frames.as_i64().unwrap_or(33) as i32)
                    .with_model_config(model_config);

                video_params.sample_params.sample_steps = steps.as_i64().unwrap_or(20) as i32;
                video_params.sample_params.guidance.txt_cfg = cfg.as_f64().unwrap_or(6.0) as f32;
                video_params.sample_params.sample_method = sample_method;
                video_params.sample_params.scheduler = scheduler_type;
                video_params.sample_params.flow_shift = flow_shift.map(|v| v as f32);

                if video_params.model_config.t5xxl_path.is_none() {
                    tracing::error!(
                        "WanVideoSampler: Wan model requires t5xxl text encoder but it is missing. \
                         Please download it to models/text_encoders/ directory."
                    );
                    return Err(ExecutorError::NodeExecutionFailed {
                        node_id: node_id.to_string(),
                        message: "Wan model requires t5xxl text encoder but it is missing.".to_string(),
                    });
                }

                match backend.generate_video(video_params) {
                    Ok(video) => {
                        let frame_count = video.frame_count();
                        tracing::info!("WanVideoSampler: generated {} video frames", frame_count);
                        Ok(vec![json!({
                            "type": "video",
                            "frame_count": frame_count,
                            "fps": video.fps,
                        })])
                    }
                    Err(e) => {
                        tracing::error!("Video generation failed: {}", e);
                        Err(ExecutorError::Inference(e))
                    }
                }
            } else {
                Ok(vec![json!({
                    "type": "video",
                    "seed": seed,
                    "steps": steps,
                    "cfg": cfg,
                    "sampler": sampler_name,
                    "scheduler": scheduler,
                    "positive": positive,
                    "negative": negative,
                    "width": width,
                    "height": height,
                    "video_frames": video_frames,
                })])
            }
        })
    }));
}

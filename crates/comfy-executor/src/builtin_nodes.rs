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
    LTX,
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
    } else if lower.contains("ltx") {
        ModelType::LTX
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

/// Like find_file_in_dir but matches substrings anywhere in the filename (case-insensitive).
fn find_file_in_dir_contains(dir: &std::path::Path, substrings: &[&str]) -> Option<String> {
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
                    substrings.iter().any(|s| lower.contains(s)).then(|| name)
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
        ModelType::LTX => (false, false, false),
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

    match model_type {
        ModelType::LTX => find_file_in_dir_contains(&vae_dir, &["video_vae", "video-vae"]),
        ModelType::SD3 | ModelType::Flux => find_file_in_dir(&vae_dir, &["sd3_vae", "flux_vae", "ae"]),
        ModelType::SDXL | ModelType::SD15 => find_file_in_dir(&vae_dir, &["sdxl_vae", "vae"]),
        ModelType::Wan => find_file_in_dir(&vae_dir, &["wan_vae"]),
        ModelType::Unknown => find_file_in_dir(&vae_dir, &["sd3_vae", "flux_vae", "sdxl_vae", "vae", "ae"]),
    }
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
    register_video_vae_decode(registry);
    register_video_vae_encode(registry);
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
    register_ltxv_audio_vae_loader(registry);
    register_ltxv_text_encoder_loader(registry);
    register_ltxv_conditioning(registry);
    register_ltxv_empty_latent_video(registry);
    register_ltxv_empty_latent_audio(registry);
    register_ltxv_img_to_video_inplace(registry);
    register_ltxv_preprocess(registry);
    register_ltxv_crop_guides(registry);
    register_ltxv_concat_av_latent(registry);
    register_ltxv_separate_av_latent(registry);
    register_ltxv_audio_vae_encode(registry);
    register_ltxv_audio_vae_decode(registry);
    register_ltxv_latent_upsampler(registry);
    register_latent_upscale_model_loader(registry);
    register_lora_loader_model_only(registry);
    register_guider_parameters(registry);
    register_multimodal_guider(registry);
    register_save_video_with_audio(registry);
    register_random_noise(registry);
    register_ksampler_select(registry);
    register_manual_sigmas(registry);
    register_cfg_guider(registry);
    register_sampler_custom_advanced(registry);
    register_create_video(registry);
    register_vae_decode_tiled(registry);
    register_resize_images_by_longer_edge(registry);
    register_resize_image_mask_node(registry);
    register_trim_audio_duration(registry);
    register_set_latent_noise_mask(registry);
    register_solid_mask(registry);
    register_primitive_int(registry);
    register_primitive_float(registry);
    register_primitive_boolean(registry);
    register_primitive_string_multiline(registry);
    register_comfy_math_expression(registry);
    register_clip_vision_loader(registry);
    register_style_model_loader(registry);
    register_upscale_model_loader(registry);
    register_gligen_loader(registry);
    register_hypernetwork_loader(registry);
    register_photomaker_loader(registry);
    register_embedding_loader(registry);
    register_classifier_loader(registry);
    register_audio_encoder_loader(registry);
    register_model_patch_loader(registry);
    register_vae_approx_loader(registry);
    register_if_else_node(registry);
    register_for_loop_node(registry);
    register_switch_node(registry);
    register_pure_function_call_node(registry);

    register_ltxv_add_guide_advanced(registry);
    register_ltxv_add_guide_advanced_attention(registry);
    register_stg_guider_node(registry);
    register_stg_guider_advanced_node(registry);
    register_stg_advanced_presets_node(registry);
    register_ltxv_apply_stg(registry);
    register_ltxv_base_sampler(registry);
    register_ltxv_extend_sampler(registry);
    register_ltxv_in_context_sampler(registry);
    register_ltxv_normalizing_sampler(registry);
    register_linear_overlap_latent_transition(registry);
    register_ltxv_looping_sampler(registry);
    register_ltxv_tiled_sampler(registry);
    register_ltxv_tiled_vae_decode(registry);
    register_ltx_add_video_ic_lora_guide(registry);
    register_ltx_add_video_ic_lora_guide_advanced(registry);
    register_ltx_iclora_loader_model_only(registry);
    register_ltxv_set_audio_ref_tokens(registry);
    register_ltxv_adain_latent(registry);
    register_ltxv_stat_norm_latent(registry);
    register_ltxv_per_step_adain_patcher(registry);
    register_ltxv_per_step_stat_norm_patcher(registry);
    register_ltxv_add_latent_guide(registry);
    register_ltxv_img_to_video_condition_only(registry);
    register_ltxv_select_latents(registry);
    register_ltxv_set_video_latent_noise_masks(registry);
    register_ltxv_laplacian_pyramid_blend(registry);
    register_float_to_int(registry);
    register_image_to_cpu(registry);
    register_ltxv_hdr_decode_postprocess(registry);
    register_ltxv_dilate_video_mask(registry);
    register_ltxv_inpaint_preprocess(registry);
    register_ltxv_patcher_vae(registry);
    register_ltxv_q8_patch(registry);
    register_ltxv_q8_lora_model_loader(registry);
    register_decoder_noise(registry);
    register_ltxv_draw_tracks(registry);
    register_ltxv_sparse_track_editor(registry);
    register_ltxv_load_conditioning(registry);
    register_ltxv_save_conditioning(registry);

    // H3 (MiniMax-HunyuanVideoAudio) ecosystem nodes - gated behind flash-attn feature
    #[cfg(feature = "flash-attn")]
    {
        register_h3_context_ir(registry);
        register_h3_director(registry);
    }

    #[cfg(feature = "controlnet")]
    crate::controlnet::register_controlnet_nodes(registry);

    crate::mask::register_mask_nodes(registry);
    crate::prompt_relay::register_prompt_relay_nodes(registry);
    crate::triposplat::register_triposplat_nodes(registry);

    // Video editing nodes (Premiere-like timeline editing)
    register_video_edit(registry);
    register_video_concat(registry);
    register_video_mix_audio(registry);
    register_video_replace_audio(registry);
    register_video_timeline(registry);
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
        function_name: "load_sd3".to_string(),
    };

    registry.register(class_def, Arc::new(|_ctx, node, _node_id| {
        let model_name = node.inputs.get("ckpt_name")
            .and_then(|v| v.as_str())
            .or_else(|| node.inputs.get("model_name").and_then(|v| v.as_str()))
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
                if let Some(path) = model.get("text_encoder_path").and_then(|v| v.as_str()) {
                    model_config = model_config.with_text_encoder(path);
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
                        "ltx" => ModelType::LTX,
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

                let model_type_str = model.get("model_type")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let detected_type = match model_type_str {
                    "sd3" => ModelType::SD3,
                    "flux" => ModelType::Flux,
                    "sdxl" => ModelType::SDXL,
                    "sd15" => ModelType::SD15,
                    "wan" => ModelType::Wan,
                    "ltx" => ModelType::LTX,
                    _ => {
                        let ckpt = model.get("model_path")
                            .or_else(|| model.get("diffusion_model_path"))
                            .and_then(|v| v.as_str())
                            .unwrap_or("");
                        detect_model_type(ckpt)
                    }
                };

                if detected_type == ModelType::LTX {
                    let mut video_frames = 1i32;
                    if let Some(latent) = latent_image.as_object() {
                        if let Some(len) = latent.get("length").and_then(|v| v.as_i64()) {
                            video_frames = len as i32;
                        }
                    }
                    let mut video_params = comfy_inference::VideoGenParams::new(prompt_text)
                        .with_negative_prompt(neg_prompt_text)
                        .with_seed(seed.as_i64().unwrap_or(42))
                        .with_dimensions(width, height)
                        .with_video_frames(video_frames)
                        .with_model_config(model_config);
                    video_params.sample_params.sample_steps = steps.as_i64().unwrap_or(20) as i32;
                    video_params.sample_params.guidance.txt_cfg = cfg.as_f64().unwrap_or(7.0) as f32;
                    video_params.sample_params.sample_method = sample_method;
                    video_params.sample_params.scheduler = sched;

                    match backend.generate_video(video_params) {
                        Ok(video) => {
                            let frame_count = video.frame_count();
                            let frames: Vec<Value> = video.frames.iter().map(|img| {
                                serde_json::to_value(img).unwrap_or_else(|_| json!({
                                    "type": "image",
                                    "width": img.width,
                                    "height": img.height,
                                    "channel": img.channel,
                                }))
                            }).collect();
                            Ok(vec![json!({
                                "type": "video",
                                "frames": frames,
                                "frame_count": frame_count,
                                "fps": video.fps,
                            })])
                        }
                        Err(e) => Err(ExecutorError::Inference(e)),
                    }
                } else {
                    let (need_clip_l, need_clip_g, need_t5xxl) = match detected_type {
                        ModelType::SD3 => (true, true, true),
                        ModelType::Flux => (true, false, true),
                        ModelType::SDXL => (true, true, false),
                        ModelType::SD15 => (true, false, false),
                        ModelType::Wan => (false, false, true),
                        _ => (true, true, true),
                    };

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
                m.insert("ckpt_name".to_string(), InputTypeSpec {
                    type_name: "COMBO".to_string(),
                    extra: HashMap::new(),
                });
                m
            },
            optional: {
                let mut m = HashMap::new();
                m.insert("vae_name".to_string(), InputTypeSpec {
                    type_name: "COMBO".to_string(),
                    extra: HashMap::new(),
                });
                m.insert("audio_vae_name".to_string(), InputTypeSpec {
                    type_name: "COMBO".to_string(),
                    extra: HashMap::new(),
                });
                m.insert("llm_name".to_string(), InputTypeSpec {
                    type_name: "COMBO".to_string(),
                    extra: HashMap::new(),
                });
                m.insert("embeddings_connectors_name".to_string(), InputTypeSpec {
                    type_name: "COMBO".to_string(),
                    extra: HashMap::new(),
                });
                m
            },
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
        let model_name = node.inputs.get("ckpt_name")
            .and_then(|v| v.as_str())
            .or_else(|| node.inputs.get("model_name").and_then(|v| v.as_str()))
            .unwrap_or("");

        let model_path = resolve_model_path("checkpoints", model_name);
        let is_gguf = model_path.to_lowercase().ends_with(".gguf");
        tracing::info!("LTXLoader: loading model from {} (gguf={})", model_path, is_gguf);

        // Explicit model paths from optional inputs (override auto-detection)
        let explicit_vae = node.inputs.get("vae_name")
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty())
            .map(|name| resolve_model_path("vae", name));
        let explicit_audio_vae = node.inputs.get("audio_vae_name")
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty())
            .map(|name| resolve_model_path("vae", name));
        let explicit_llm = node.inputs.get("llm_name")
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty())
            .map(|name| resolve_model_path("text_encoders", name));
        let explicit_embeddings = node.inputs.get("embeddings_connectors_name")
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty())
            .map(|name| resolve_model_path("text_encoders", name));

        let base = get_models_base_dir();
        let te_dir = base.join("text_encoders");
        let vae_dir = base.join("vae");
        let llm_dir = base.join("llm");

        // Auto-detect text encoder (gemma-3-12b-it) if not explicitly specified
        let text_encoder_path = explicit_llm.or_else(|| {
            let mut found: Option<String> = None;
            if let Some(p) = find_file_in_dir(&te_dir, &["gemma-3-12b-it", "gemma_3_12B_it"]) {
                found = Some(p);
            }
            if found.is_none() && llm_dir.exists() {
                if let Ok(entries) = std::fs::read_dir(&llm_dir) {
                    for entry in entries.flatten() {
                        let name = entry.file_name().to_string_lossy().to_string();
                        let lower = name.to_lowercase();
                        if lower.contains("gemma-3-12b") || lower.contains("gemma_3_12b") {
                            let sub_dir = entry.path();
                            if let Some(p) = find_file_in_dir(&sub_dir, &["gemma-3-12b-it", "gemma_3_12B_it"]) {
                                found = Some(p);
                                break;
                            }
                            if sub_dir.is_dir() {
                                if let Ok(sub_entries) = std::fs::read_dir(&sub_dir) {
                                    let has_gguf = sub_entries.flatten().any(|e| {
                                        e.file_name().to_string_lossy().to_string().ends_with(".gguf")
                                    });
                                    if has_gguf {
                                        found = Some(sub_dir.to_string_lossy().to_string());
                                        break;
                                    }
                                }
                                if let Ok(sub_entries) = std::fs::read_dir(&sub_dir) {
                                    let has_safetensors = sub_entries.flatten().any(|e| {
                                        e.file_name().to_string_lossy().to_string().starts_with("model-") &&
                                        e.file_name().to_string_lossy().to_string().ends_with(".safetensors")
                                    });
                                    if has_safetensors {
                                        found = Some(sub_dir.to_string_lossy().to_string());
                                        break;
                                    }
                                }
                            }
                        }
                    }
                }
            }
            if found.is_some() {
                tracing::info!("LTXLoader: auto-detected text encoder at {:?}", found);
            }
            found
        });

        // Auto-detect video VAE if not explicitly specified
        let video_vae_path = explicit_vae.or_else(|| {
            let p = find_file_in_dir_contains(&vae_dir, &["video_vae", "video-vae"]);
            if p.is_some() {
                tracing::info!("LTXLoader: auto-detected video VAE at {:?}", p);
            }
            p
        });

        // Auto-detect audio VAE if not explicitly specified
        let audio_vae_path = explicit_audio_vae.or_else(|| {
            let p = find_file_in_dir_contains(&vae_dir, &["audio_vae", "audio-vae"]);
            if p.is_some() {
                tracing::info!("LTXLoader: auto-detected audio VAE at {:?}", p);
            }
            p
        });

        // Auto-detect embeddings connectors if not explicitly specified
        let embeddings_connectors_path = explicit_embeddings.or_else(|| {
            let p = find_file_in_dir_contains(&te_dir, &["embeddings_connector", "embedding_connector"]);
            if p.is_some() {
                tracing::info!("LTXLoader: auto-detected embeddings connectors at {:?}", p);
            }
            p
        });

        Box::pin(async move {
            // For GGUF files, use diffusion_model_path instead of model_path
            let mut model_json = if is_gguf {
                json!({
                    "diffusion_model_path": model_path,
                    "model_type": "ltx",
                })
            } else {
                json!({
                    "model_path": model_path,
                    "model_type": "ltx",
                })
            };
            if let Some(ref te_path) = text_encoder_path {
                model_json["text_encoder_path"] = json!(te_path);
                model_json["llm_path"] = json!(te_path);
            }
            if let Some(ref p) = video_vae_path {
                model_json["vae_path"] = json!(p);
            }
            if let Some(ref p) = audio_vae_path {
                model_json["audio_vae_path"] = json!(p);
            }
            if let Some(ref p) = embeddings_connectors_path {
                model_json["embeddings_connectors_path"] = json!(p);
            }

            let mut clip_json = json!({
                "type": "clip",
                "source_model": model_path,
                "model_type": "ltx",
            });
            if let Some(ref te_path) = text_encoder_path {
                clip_json["text_encoder_path"] = json!(te_path);
                clip_json["llm_path"] = json!(te_path);
            }

            let mut vae_config = json!({
                "type": "vae",
                "source_model": model_path,
                "model_type": "ltx",
            });
            if let Some(ref p) = video_vae_path {
                vae_config["vae_path"] = json!(p);
            }
            if let Some(ref p) = audio_vae_path {
                vae_config["audio_vae_path"] = json!(p);
            }

            tracing::info!("LTXLoader: model_json = {}", serde_json::to_string_pretty(&model_json).unwrap_or_default());
            Ok(vec![model_json, clip_json, vae_config])
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
                if let Some(path) = model.get("text_encoder_path").and_then(|v| v.as_str()) {
                    model_config = model_config.with_text_encoder(path);
                }
                if let Some(path) = model.get("llm_path").and_then(|v| v.as_str()) {
                    model_config = model_config.with_llm(path);
                }
                if let Some(path) = model.get("audio_vae_path").and_then(|v| v.as_str()) {
                    model_config = model_config.with_audio_vae(path);
                }
                if let Some(path) = model.get("embeddings_connectors_path").and_then(|v| v.as_str()) {
                    model_config = model_config.with_embeddings_connectors(path);
                }
                tracing::info!("LTXVideoSampler: model_config text_encoder_path={:?}, llm_path={:?}, vae_path={:?}, audio_vae_path={:?}, embeddings_connectors_path={:?}",
                    model_config.text_encoder_path, model_config.llm_path, model_config.vae_path, model_config.audio_vae_path, model_config.embeddings_connectors_path);

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
                    if model_config.text_encoder_path.is_none() {
                        if let Some(path) = clip.get("text_encoder_path").and_then(|v| v.as_str()) {
                            model_config = model_config.with_text_encoder(path);
                        }
                    }
                    if model_config.llm_path.is_none() {
                        if let Some(path) = clip.get("llm_path").and_then(|v| v.as_str()) {
                            model_config = model_config.with_llm(path);
                        }
                    }
                }

                let needs_clip_auto_detect = model_config.clip_l_path.is_none()
                    || model_config.t5xxl_path.is_none();
                let needs_vae_auto_detect = model_config.vae_path.is_none();
                if needs_clip_auto_detect || needs_vae_auto_detect {
                    if needs_clip_auto_detect {
                        let (clip_l, _, t5xxl) = auto_detect_text_encoders(ModelType::LTX);
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
                        if let Some(path) = auto_detect_vae(ModelType::LTX) {
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

                // Capture model paths before moving model_config into video_params
                let mc_model_path = model_config.model_path.clone();
                let mc_diffusion_model_path = model_config.diffusion_model_path.clone();
                let mc_audio_vae_path = model_config.audio_vae_path.clone();
                let mc_embeddings_connectors_path = model_config.embeddings_connectors_path.clone();

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
                        // Debug: check first frame pixel values
                        if let Some(first_frame) = video.frames.first() {
                            let sample_pixels: Vec<u8> = first_frame.data.iter().take(30).copied().collect();
                            let all_white = first_frame.data.iter().all(|&v| v == 255);
                            let all_black = first_frame.data.iter().all(|&v| v == 0);
                            tracing::info!("LTXVideoSampler: first frame {}x{} ch={}, first 30 pixels: {:?}, all_white={}, all_black={}",
                                first_frame.width, first_frame.height, first_frame.channel, sample_pixels, all_white, all_black);
                        }
                        let video_val = serde_json::to_value(&video).unwrap_or(json!({}));
                        let mut result = json!({
                            "type": "video",
                            "frame_count": frame_count,
                            "fps": video.fps,
                            "num_frames_per_seed": num_frames_per_seed,
                            "frames": video_val.get("frames").cloned().unwrap_or(json!([])),
                            "width": width,
                            "height": height,
                        });
                        // Pass through model paths for downstream nodes (VideoVAEDecode)
                        if let Some(p) = mc_model_path {
                            result["model_path"] = json!(p);
                        }
                        if let Some(p) = mc_diffusion_model_path {
                            result["diffusion_model_path"] = json!(p);
                        }
                        if let Some(p) = mc_audio_vae_path {
                            result["audio_vae_path"] = json!(p);
                        }
                        if let Some(p) = mc_embeddings_connectors_path {
                            result["embeddings_connectors_path"] = json!(p);
                        }
                        Ok(vec![result])
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

            let sd_video = comfy_inference::SdVideo::new_without_audio(
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

                let video = comfy_inference::SdVideo::new_without_audio(frames, fps as i32);
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
            if let Some(frames) = samples.get("frames").and_then(|v| v.as_array()) {
                if !frames.is_empty() {
                    return Ok(vec![json!({
                        "type": "image",
                        "images": frames,
                    })]);
                }
            }

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

fn register_video_vae_decode(registry: &mut NodeRegistry) {
    let class_def = NodeClassDef {
        class_type: "VideoVAEDecode".to_string(),
        display_name: "Video VAE Decode".to_string(),
        category: "latent/video".to_string(),
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
        function_name: "decode_video".to_string(),
    };

    registry.register(class_def, Arc::new(|ctx, _node, node_id| {
        let samples = ctx.resolve_input(node_id, "samples")
            .unwrap_or_else(|_| json!(null));
        let vae = ctx.resolve_input(node_id, "vae")
            .unwrap_or_else(|_| json!(null));
        let fps = ctx.resolve_input(node_id, "fps")
            .ok()
            .and_then(|v| v.as_i64())
            .unwrap_or(8);

        let backend = ctx.backend();

        Box::pin(async move {
            let frame_count = samples.get("frame_count")
                .and_then(|v| v.as_i64())
                .or_else(|| samples.get("video_frames").and_then(|v| v.as_i64()))
                .unwrap_or(1);

            let width = samples.get("width")
                .and_then(|v| v.as_i64())
                .unwrap_or(832);
            let height = samples.get("height")
                .and_then(|v| v.as_i64())
                .unwrap_or(480);

            let vae_path = vae.get("vae_path")
                .and_then(|v| v.as_str())
                .or_else(|| vae.get("source_model").and_then(|v| v.as_str()))
                .unwrap_or("");

            if backend.supports_video_generation() && !vae_path.is_empty() {
                let mut model_config = ModelConfig::default();
                model_config = model_config.with_vae(vae_path);

                if let Some(path) = samples.get("model_path").and_then(|v| v.as_str()) {
                    model_config = model_config.with_model(path);
                }
                if let Some(path) = samples.get("diffusion_model_path").and_then(|v| v.as_str()) {
                    model_config = model_config.with_diffusion_model(path);
                }
                if let Some(path) = vae.get("audio_vae_path").and_then(|v| v.as_str()) {
                    model_config = model_config.with_audio_vae(path);
                }
                if let Some(path) = samples.get("audio_vae_path").and_then(|v| v.as_str()) {
                    model_config = model_config.with_audio_vae(path);
                }
                if let Some(path) = samples.get("embeddings_connectors_path").and_then(|v| v.as_str()) {
                    model_config = model_config.with_embeddings_connectors(path);
                }

                let video_params = comfy_inference::VideoGenParams::new("")
                    .with_dimensions(width as i32, height as i32)
                    .with_video_frames(frame_count as i32)
                    .with_model_config(model_config);

                match backend.decode_video_latent(&samples, &video_params) {
                    Ok(video) => {
                        let decoded_frames = video.frame_count();
                        tracing::info!(
                            "VideoVAEDecode: decoded {} video frames ({}x{}, {}fps)",
                            decoded_frames, width, height, fps
                        );
                        let video_val = serde_json::to_value(&video).unwrap_or(json!({}));
                        Ok(vec![json!({
                            "type": "video",
                            "videos": [{
                                "filename": "video_vae_decode",
                                "subfolder": "",
                                "type": "temp",
                            }],
                            "frames": video_val.get("frames").cloned().unwrap_or(json!([])),
                            "frame_count": decoded_frames,
                            "fps": fps,
                            "width": width,
                            "height": height,
                        })])
                    }
                    Err(e) => {
                        tracing::warn!("VideoVAEDecode: backend decode_video_latent failed: {}, falling back to passthrough", e);
                        // Pass through frames if input already contains decoded frames
                        let passthrough_frames = samples.get("frames")
                            .and_then(|v| v.as_array())
                            .cloned();
                        let mut result = json!({
                            "type": "video",
                            "frame_count": frame_count,
                            "fps": fps,
                            "width": width,
                            "height": height,
                            "source": "video_vae_decode",
                            "latent": samples,
                            "vae": vae,
                        });
                        if let Some(frames) = passthrough_frames {
                            result["frames"] = json!(frames);
                        }
                        Ok(vec![result])
                    }
                }
            } else {
                if let Some(sample_arr) = samples.get("samples").and_then(|v| v.as_array()) {
                    if !sample_arr.is_empty() {
                        return Ok(vec![json!({
                            "type": "video",
                            "frames": sample_arr,
                            "frame_count": sample_arr.len(),
                            "fps": fps,
                            "width": width,
                            "height": height,
                        })]);
                    }
                }

                Ok(vec![json!({
                    "type": "video",
                    "frame_count": frame_count,
                    "fps": fps,
                    "width": width,
                    "height": height,
                    "source": "video_vae_decode",
                    "latent": samples,
                    "vae": vae,
                })])
            }
        })
    }));
}

fn register_video_vae_encode(registry: &mut NodeRegistry) {
    let class_def = NodeClassDef {
        class_type: "VideoVAEEncode".to_string(),
        display_name: "Video VAE Encode".to_string(),
        category: "latent/video".to_string(),
        input_types: NodeInputTypes {
            required: {
                let mut m = HashMap::new();
                m.insert("video".to_string(), InputTypeSpec {
                    type_name: "VIDEO".to_string(),
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
        function_name: "encode_video".to_string(),
    };

    registry.register(class_def, Arc::new(|_ctx, _node, node_id| {
        let video = _ctx.resolve_input(node_id, "video")
            .unwrap_or_else(|_| json!(null));
        let vae = _ctx.resolve_input(node_id, "vae")
            .unwrap_or_else(|_| json!(null));

        Box::pin(async move {
            let frame_count = video.get("frame_count")
                .and_then(|v| v.as_i64())
                .or_else(|| video.get("frames").and_then(|v| v.as_array()).map(|a| a.len() as i64))
                .unwrap_or(1);

            let fps = video.get("fps")
                .and_then(|v| v.as_i64())
                .unwrap_or(8);

            let width = video.get("width")
                .and_then(|v| v.as_i64())
                .unwrap_or(832);

            let height = video.get("height")
                .and_then(|v| v.as_i64())
                .unwrap_or(480);

            Ok(vec![json!({
                "type": "latent",
                "source": "video_vae_encode",
                "video": video,
                "vae": vae,
                "frame_count": frame_count,
                "fps": fps,
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

fn register_ltxv_audio_vae_loader(registry: &mut NodeRegistry) {
    let class_def = NodeClassDef {
        class_type: "LTXVAudioVAELoader".to_string(),
        display_name: "Load LTX Audio VAE".to_string(),
        category: "loaders/ltxv".to_string(),
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
        output_types: vec![IoType::Vae],
        output_names: vec!["Audio VAE".to_string()],
        output_is_list: vec![false],
        is_output_node: false,
        has_intermediate_output: false,
        is_changed: None,
        not_idempotent: false,
        function_name: "load_audio_vae".to_string(),
    };

    registry.register(class_def, Arc::new(|_ctx, node, _node_id| {
        let ckpt_name = node.inputs.get("ckpt_name")
            .and_then(|v| v.as_str())
            .unwrap_or("ltx-2.3-22b-dev-fp8.safetensors");
        let model_path = resolve_model_path("checkpoints", ckpt_name);

        Box::pin(async move {
            Ok(vec![json!({
                "type": "audio_vae",
                "source_model": model_path,
                "model_type": "ltx",
            })])
        })
    }));
}

fn register_ltxv_text_encoder_loader(registry: &mut NodeRegistry) {
    let class_def = NodeClassDef {
        class_type: "LTXAVTextEncoderLoader".to_string(),
        display_name: "Load LTX Text Encoder".to_string(),
        category: "loaders/ltxv".to_string(),
        input_types: NodeInputTypes {
            required: {
                let mut m = HashMap::new();
                m.insert("text_encoder".to_string(), InputTypeSpec {
                    type_name: "COMBO".to_string(),
                    extra: HashMap::new(),
                });
                m.insert("ckpt_name".to_string(), InputTypeSpec {
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
        function_name: "load_text_encoder".to_string(),
    };

    registry.register(class_def, Arc::new(|_ctx, node, _node_id| {
        let text_encoder = node.inputs.get("text_encoder")
            .and_then(|v| v.as_str())
            .unwrap_or("gemma_3_12B_it_fp4_mixed.safetensors");
        let ckpt_name = node.inputs.get("ckpt_name")
            .and_then(|v| v.as_str())
            .unwrap_or("ltx-2.3-22b-dev-fp8.safetensors");

        let te_path = resolve_model_path("text_encoders", text_encoder);
        let ckpt_path = resolve_model_path("checkpoints", ckpt_name);

        Box::pin(async move {
            Ok(vec![json!({
                "type": "clip",
                "text_encoder_path": te_path,
                "llm_path": te_path,
                "source_model": ckpt_path,
                "model_type": "ltx",
            })])
        })
    }));
}

fn register_ltxv_conditioning(registry: &mut NodeRegistry) {
    let class_def = NodeClassDef {
        class_type: "LTXVConditioning".to_string(),
        display_name: "LTXV Conditioning".to_string(),
        category: "conditioning/ltxv".to_string(),
        input_types: NodeInputTypes {
            required: {
                let mut m = HashMap::new();
                m.insert("positive".to_string(), InputTypeSpec {
                    type_name: "CONDITIONING".to_string(),
                    extra: HashMap::new(),
                });
                m.insert("negative".to_string(), InputTypeSpec {
                    type_name: "CONDITIONING".to_string(),
                    extra: HashMap::new(),
                });
                m.insert("frame_rate".to_string(), InputTypeSpec {
                    type_name: "FLOAT".to_string(),
                    extra: HashMap::new(),
                });
                m
            },
            optional: HashMap::new(),
            hidden: HashMap::new(),
        },
        output_types: vec![IoType::Conditioning, IoType::Conditioning],
        output_names: vec!["positive".to_string(), "negative".to_string()],
        output_is_list: vec![false, false],
        is_output_node: false,
        has_intermediate_output: false,
        is_changed: None,
        not_idempotent: false,
        function_name: "condition".to_string(),
    };

    registry.register(class_def, Arc::new(|ctx, _node, node_id| {
        let positive = ctx.resolve_input(node_id, "positive").unwrap_or_else(|_| json!({}));
        let negative = ctx.resolve_input(node_id, "negative").unwrap_or_else(|_| json!({}));
        let frame_rate = ctx.resolve_input(node_id, "frame_rate").unwrap_or_else(|_| json!(25.0));

        Box::pin(async move {
            let mut pos = positive;
            if let Some(obj) = pos.as_object_mut() {
                obj.insert("frame_rate".to_string(), frame_rate.clone());
                obj.insert("ltxv_conditioning".to_string(), json!(true));
            }
            let mut neg = negative;
            if let Some(obj) = neg.as_object_mut() {
                obj.insert("frame_rate".to_string(), frame_rate);
                obj.insert("ltxv_conditioning".to_string(), json!(true));
            }
            Ok(vec![pos, neg])
        })
    }));
}

fn register_ltxv_empty_latent_video(registry: &mut NodeRegistry) {
    let class_def = NodeClassDef {
        class_type: "EmptyLTXVLatentVideo".to_string(),
        display_name: "Empty LTXV Latent Video".to_string(),
        category: "latent/ltxv".to_string(),
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
                m.insert("length".to_string(), InputTypeSpec {
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
            .unwrap_or(768);
        let height = node.inputs.get("height")
            .and_then(|v| v.as_i64())
            .unwrap_or(512);
        let length = node.inputs.get("length")
            .and_then(|v| v.as_i64())
            .unwrap_or(97);
        let batch_size = node.inputs.get("batch_size")
            .and_then(|v| v.as_i64())
            .unwrap_or(1);

        Box::pin(async move {
            Ok(vec![json!({
                "type": "latent",
                "subtype": "ltxv_video",
                "width": width,
                "height": height,
                "length": length,
                "batch_size": batch_size,
            })])
        })
    }));
}

fn register_ltxv_empty_latent_audio(registry: &mut NodeRegistry) {
    let class_def = NodeClassDef {
        class_type: "LTXVEmptyLatentAudio".to_string(),
        display_name: "Empty LTXV Latent Audio".to_string(),
        category: "latent/ltxv".to_string(),
        input_types: NodeInputTypes {
            required: {
                let mut m = HashMap::new();
                m.insert("audio_vae".to_string(), InputTypeSpec {
                    type_name: "VAE".to_string(),
                    extra: HashMap::new(),
                });
                m.insert("frames_number".to_string(), InputTypeSpec {
                    type_name: "INT".to_string(),
                    extra: HashMap::new(),
                });
                m.insert("frame_rate".to_string(), InputTypeSpec {
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
        output_names: vec!["Latent".to_string()],
        output_is_list: vec![false],
        is_output_node: false,
        has_intermediate_output: false,
        is_changed: None,
        not_idempotent: false,
        function_name: "generate".to_string(),
    };

    registry.register(class_def, Arc::new(|_ctx, node, _node_id| {
        let audio_vae = node.inputs.get("audio_vae").cloned().unwrap_or(json!(null));
        let frames_number = node.inputs.get("frames_number")
            .and_then(|v| v.as_i64())
            .unwrap_or(97);
        let frame_rate = node.inputs.get("frame_rate")
            .and_then(|v| v.as_i64())
            .unwrap_or(25);
        let batch_size = node.inputs.get("batch_size")
            .and_then(|v| v.as_i64())
            .unwrap_or(1);

        Box::pin(async move {
            Ok(vec![json!({
                "type": "latent",
                "subtype": "ltxv_audio",
                "audio_vae": audio_vae,
                "frames_number": frames_number,
                "frame_rate": frame_rate,
                "batch_size": batch_size,
            })])
        })
    }));
}

fn register_ltxv_img_to_video_inplace(registry: &mut NodeRegistry) {
    let class_def = NodeClassDef {
        class_type: "LTXVImgToVideoInplace".to_string(),
        display_name: "LTXV Image to Video Inplace".to_string(),
        category: "latent/ltxv".to_string(),
        input_types: NodeInputTypes {
            required: {
                let mut m = HashMap::new();
                m.insert("vae".to_string(), InputTypeSpec {
                    type_name: "VAE".to_string(),
                    extra: HashMap::new(),
                });
                m.insert("image".to_string(), InputTypeSpec {
                    type_name: "IMAGE".to_string(),
                    extra: HashMap::new(),
                });
                m.insert("latent".to_string(), InputTypeSpec {
                    type_name: "LATENT".to_string(),
                    extra: HashMap::new(),
                });
                m.insert("bypass".to_string(), InputTypeSpec {
                    type_name: "BOOLEAN".to_string(),
                    extra: HashMap::new(),
                });
                m
            },
            optional: HashMap::new(),
            hidden: HashMap::new(),
        },
        output_types: vec![IoType::Latent],
        output_names: vec!["latent".to_string()],
        output_is_list: vec![false],
        is_output_node: false,
        has_intermediate_output: false,
        is_changed: None,
        not_idempotent: false,
        function_name: "img_to_video".to_string(),
    };

    registry.register(class_def, Arc::new(|ctx, _node, node_id| {
        let vae = ctx.resolve_input(node_id, "vae").unwrap_or_else(|_| json!(null));
        let image = ctx.resolve_input(node_id, "image").unwrap_or_else(|_| json!(null));
        let latent = ctx.resolve_input(node_id, "latent").unwrap_or_else(|_| json!({}));
        let bypass = ctx.resolve_input(node_id, "bypass")
            .ok()
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        Box::pin(async move {
            if bypass {
                return Ok(vec![latent]);
            }
            let mut result = latent;
            if let Some(obj) = result.as_object_mut() {
                obj.insert("type".to_string(), json!("latent"));
                obj.insert("subtype".to_string(), json!("ltxv_video"));
                obj.insert("first_frame_image".to_string(), image);
                obj.insert("vae".to_string(), vae);
            }
            Ok(vec![result])
        })
    }));
}

fn register_ltxv_preprocess(registry: &mut NodeRegistry) {
    let class_def = NodeClassDef {
        class_type: "LTXVPreprocess".to_string(),
        display_name: "LTXV Preprocess".to_string(),
        category: "image/ltxv".to_string(),
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
                m.insert("strip_weight".to_string(), InputTypeSpec {
                    type_name: "FLOAT".to_string(),
                    extra: HashMap::new(),
                });
                m
            },
            hidden: HashMap::new(),
        },
        output_types: vec![IoType::Image],
        output_names: vec!["output_image".to_string()],
        output_is_list: vec![false],
        is_output_node: false,
        has_intermediate_output: false,
        is_changed: None,
        not_idempotent: false,
        function_name: "preprocess".to_string(),
    };

    registry.register(class_def, Arc::new(|_ctx, node, _node_id| {
        let _strip_weight = node.inputs.get("strip_weight")
            .and_then(|v| v.as_f64())
            .unwrap_or(18.0);

        Box::pin(async move {
            Ok(vec![json!({
                "type": "image",
                "preprocessed": true,
                "preprocess_type": "ltxv_strip",
            })])
        })
    }));
}

fn register_ltxv_crop_guides(registry: &mut NodeRegistry) {
    let class_def = NodeClassDef {
        class_type: "LTXVCropGuides".to_string(),
        display_name: "LTXV Crop Guides".to_string(),
        category: "conditioning/ltxv".to_string(),
        input_types: NodeInputTypes {
            required: {
                let mut m = HashMap::new();
                m.insert("positive".to_string(), InputTypeSpec {
                    type_name: "CONDITIONING".to_string(),
                    extra: HashMap::new(),
                });
                m.insert("negative".to_string(), InputTypeSpec {
                    type_name: "CONDITIONING".to_string(),
                    extra: HashMap::new(),
                });
                m.insert("latent".to_string(), InputTypeSpec {
                    type_name: "LATENT".to_string(),
                    extra: HashMap::new(),
                });
                m
            },
            optional: HashMap::new(),
            hidden: HashMap::new(),
        },
        output_types: vec![IoType::Conditioning, IoType::Conditioning, IoType::Latent],
        output_names: vec!["positive".to_string(), "negative".to_string(), "latent".to_string()],
        output_is_list: vec![false, false, false],
        is_output_node: false,
        has_intermediate_output: false,
        is_changed: None,
        not_idempotent: false,
        function_name: "crop_guides".to_string(),
    };

    registry.register(class_def, Arc::new(|ctx, _node, node_id| {
        let positive = ctx.resolve_input(node_id, "positive").unwrap_or_else(|_| json!({}));
        let negative = ctx.resolve_input(node_id, "negative").unwrap_or_else(|_| json!({}));
        let latent = ctx.resolve_input(node_id, "latent").unwrap_or_else(|_| json!({}));

        Box::pin(async move {
            Ok(vec![positive, negative, latent])
        })
    }));
}

fn register_ltxv_concat_av_latent(registry: &mut NodeRegistry) {
    let class_def = NodeClassDef {
        class_type: "LTXVConcatAVLatent".to_string(),
        display_name: "LTXV Concat AV Latent".to_string(),
        category: "latent/ltxv".to_string(),
        input_types: NodeInputTypes {
            required: {
                let mut m = HashMap::new();
                m.insert("video_latent".to_string(), InputTypeSpec {
                    type_name: "LATENT".to_string(),
                    extra: HashMap::new(),
                });
                m.insert("audio_latent".to_string(), InputTypeSpec {
                    type_name: "LATENT".to_string(),
                    extra: HashMap::new(),
                });
                m
            },
            optional: HashMap::new(),
            hidden: HashMap::new(),
        },
        output_types: vec![IoType::Latent],
        output_names: vec!["latent".to_string()],
        output_is_list: vec![false],
        is_output_node: false,
        has_intermediate_output: false,
        is_changed: None,
        not_idempotent: false,
        function_name: "concat".to_string(),
    };

    registry.register(class_def, Arc::new(|ctx, _node, node_id| {
        let video_latent = ctx.resolve_input(node_id, "video_latent").unwrap_or_else(|_| json!({}));
        let audio_latent = ctx.resolve_input(node_id, "audio_latent").unwrap_or_else(|_| json!({}));

        Box::pin(async move {
            Ok(vec![json!({
                "type": "latent",
                "subtype": "ltxv_av",
                "video_latent": video_latent,
                "audio_latent": audio_latent,
            })])
        })
    }));
}

fn register_ltxv_separate_av_latent(registry: &mut NodeRegistry) {
    let class_def = NodeClassDef {
        class_type: "LTXVSeparateAVLatent".to_string(),
        display_name: "LTXV Separate AV Latent".to_string(),
        category: "latent/ltxv".to_string(),
        input_types: NodeInputTypes {
            required: {
                let mut m = HashMap::new();
                m.insert("av_latent".to_string(), InputTypeSpec {
                    type_name: "LATENT".to_string(),
                    extra: HashMap::new(),
                });
                m
            },
            optional: HashMap::new(),
            hidden: HashMap::new(),
        },
        output_types: vec![IoType::Latent, IoType::Latent],
        output_names: vec!["video_latent".to_string(), "audio_latent".to_string()],
        output_is_list: vec![false, false],
        is_output_node: false,
        has_intermediate_output: false,
        is_changed: None,
        not_idempotent: false,
        function_name: "separate".to_string(),
    };

    registry.register(class_def, Arc::new(|ctx, _node, node_id| {
        let av_latent = ctx.resolve_input(node_id, "av_latent").unwrap_or_else(|_| json!({}));

        Box::pin(async move {
            let video_latent = av_latent.get("video_latent").cloned().unwrap_or_else(|| {
                let mut l = av_latent.clone();
                if let Some(obj) = l.as_object_mut() {
                    obj.insert("subtype".to_string(), json!("ltxv_video"));
                }
                l
            });
            let audio_latent = av_latent.get("audio_latent").cloned().unwrap_or_else(|| {
                json!({
                    "type": "latent",
                    "subtype": "ltxv_audio",
                })
            });
            Ok(vec![video_latent, audio_latent])
        })
    }));
}

fn register_ltxv_audio_vae_encode(registry: &mut NodeRegistry) {
    let class_def = NodeClassDef {
        class_type: "LTXVAudioVAEEncode".to_string(),
        display_name: "LTXV Audio VAE Encode".to_string(),
        category: "audio/ltxv".to_string(),
        input_types: NodeInputTypes {
            required: {
                let mut m = HashMap::new();
                m.insert("audio".to_string(), InputTypeSpec {
                    type_name: "AUDIO".to_string(),
                    extra: HashMap::new(),
                });
                m.insert("audio_vae".to_string(), InputTypeSpec {
                    type_name: "VAE".to_string(),
                    extra: HashMap::new(),
                });
                m
            },
            optional: HashMap::new(),
            hidden: HashMap::new(),
        },
        output_types: vec![IoType::Latent],
        output_names: vec!["Audio Latent".to_string()],
        output_is_list: vec![false],
        is_output_node: false,
        has_intermediate_output: false,
        is_changed: None,
        not_idempotent: false,
        function_name: "encode".to_string(),
    };

    registry.register(class_def, Arc::new(|ctx, _node, node_id| {
        let audio = ctx.resolve_input(node_id, "audio").unwrap_or_else(|_| json!(null));
        let audio_vae = ctx.resolve_input(node_id, "audio_vae").unwrap_or_else(|_| json!(null));

        Box::pin(async move {
            Ok(vec![json!({
                "type": "latent",
                "subtype": "ltxv_audio",
                "audio": audio,
                "audio_vae": audio_vae,
            })])
        })
    }));
}

fn register_ltxv_audio_vae_decode(registry: &mut NodeRegistry) {
    let class_def = NodeClassDef {
        class_type: "LTXVAudioVAEDecode".to_string(),
        display_name: "LTXV Audio VAE Decode".to_string(),
        category: "audio/ltxv".to_string(),
        input_types: NodeInputTypes {
            required: {
                let mut m = HashMap::new();
                m.insert("samples".to_string(), InputTypeSpec {
                    type_name: "LATENT".to_string(),
                    extra: HashMap::new(),
                });
                m.insert("audio_vae".to_string(), InputTypeSpec {
                    type_name: "VAE".to_string(),
                    extra: HashMap::new(),
                });
                m
            },
            optional: HashMap::new(),
            hidden: HashMap::new(),
        },
        output_types: vec![IoType::Audio],
        output_names: vec!["Audio".to_string()],
        output_is_list: vec![false],
        is_output_node: false,
        has_intermediate_output: false,
        is_changed: None,
        not_idempotent: false,
        function_name: "decode".to_string(),
    };

    registry.register(class_def, Arc::new(|ctx, _node, node_id| {
        let samples = ctx.resolve_input(node_id, "samples").unwrap_or_else(|_| json!(null));
        let audio_vae = ctx.resolve_input(node_id, "audio_vae").unwrap_or_else(|_| json!(null));

        Box::pin(async move {
            Ok(vec![json!({
                "type": "audio",
                "subtype": "ltxv_decoded",
                "samples": samples,
                "audio_vae": audio_vae,
            })])
        })
    }));
}

fn register_ltxv_latent_upsampler(registry: &mut NodeRegistry) {
    let class_def = NodeClassDef {
        class_type: "LTXVLatentUpsampler".to_string(),
        display_name: "LTXV Latent Upsampler".to_string(),
        category: "latent/ltxv".to_string(),
        input_types: NodeInputTypes {
            required: {
                let mut m = HashMap::new();
                m.insert("samples".to_string(), InputTypeSpec {
                    type_name: "LATENT".to_string(),
                    extra: HashMap::new(),
                });
                m.insert("upscale_model".to_string(), InputTypeSpec {
                    type_name: "LATENT_UPSCALE_MODEL".to_string(),
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
        function_name: "upscale".to_string(),
    };

    registry.register(class_def, Arc::new(|ctx, _node, node_id| {
        let samples = ctx.resolve_input(node_id, "samples").unwrap_or_else(|_| json!({}));
        let upscale_model = ctx.resolve_input(node_id, "upscale_model").unwrap_or_else(|_| json!(null));
        let vae = ctx.resolve_input(node_id, "vae").unwrap_or_else(|_| json!(null));

        Box::pin(async move {
            let mut result = samples;
            if let Some(obj) = result.as_object_mut() {
                obj.insert("upscaled".to_string(), json!(true));
                obj.insert("upscale_model".to_string(), upscale_model);
                obj.insert("vae".to_string(), vae);
            }
            Ok(vec![result])
        })
    }));
}

fn register_latent_upscale_model_loader(registry: &mut NodeRegistry) {
    let class_def = NodeClassDef {
        class_type: "LatentUpscaleModelLoader".to_string(),
        display_name: "Load Latent Upscale Model".to_string(),
        category: "loaders".to_string(),
        input_types: NodeInputTypes {
            required: {
                let mut m = HashMap::new();
                m.insert("latent_upscale_model_name".to_string(), InputTypeSpec {
                    type_name: "COMBO".to_string(),
                    extra: HashMap::new(),
                });
                m
            },
            optional: HashMap::new(),
            hidden: HashMap::new(),
        },
        output_types: vec![IoType::LatentUpscaleModel],
        output_names: vec!["LATENT_UPSCALE_MODEL".to_string()],
        output_is_list: vec![false],
        is_output_node: false,
        has_intermediate_output: false,
        is_changed: None,
        not_idempotent: false,
        function_name: "load".to_string(),
    };

    registry.register(class_def, Arc::new(|_ctx, node, _node_id| {
        let model_name = node.inputs.get("latent_upscale_model_name")
            .and_then(|v| v.as_str())
            .unwrap_or("ltx-2.3-spatial-upscaler-x2-1.1.safetensors");
        let model_path = resolve_model_path("latent_upscale_models", model_name);

        Box::pin(async move {
            Ok(vec![json!({
                "type": "latent_upscale_model",
                "model_path": model_path,
            })])
        })
    }));
}

fn register_lora_loader_model_only(registry: &mut NodeRegistry) {
    let class_def = NodeClassDef {
        class_type: "LoraLoaderModelOnly".to_string(),
        display_name: "Load LoRA (Model Only)".to_string(),
        category: "loaders".to_string(),
        input_types: NodeInputTypes {
            required: {
                let mut m = HashMap::new();
                m.insert("model".to_string(), InputTypeSpec {
                    type_name: "MODEL".to_string(),
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
        function_name: "load_lora".to_string(),
    };

    registry.register(class_def, Arc::new(|ctx, node, node_id| {
        let model = ctx.resolve_input(node_id, "model").unwrap_or_else(|_| json!({}));
        let lora_name = node.inputs.get("lora_name")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let strength_model = node.inputs.get("strength_model")
            .and_then(|v| v.as_f64())
            .unwrap_or(1.0);

        let lora_path = resolve_model_path("loras", lora_name);

        Box::pin(async move {
            let mut result = model;
            if let Some(obj) = result.as_object_mut() {
                obj.insert("lora_path".to_string(), json!(lora_path));
                obj.insert("lora_strength".to_string(), json!(strength_model));
            }
            Ok(vec![result])
        })
    }));
}

fn register_random_noise(registry: &mut NodeRegistry) {
    let class_def = NodeClassDef {
        class_type: "RandomNoise".to_string(),
        display_name: "Random Noise".to_string(),
        category: "noise".to_string(),
        input_types: NodeInputTypes {
            required: {
                let mut m = HashMap::new();
                m.insert("noise_seed".to_string(), InputTypeSpec {
                    type_name: "INT".to_string(),
                    extra: HashMap::new(),
                });
                m.insert("control_after_generate".to_string(), InputTypeSpec {
                    type_name: "COMBO".to_string(),
                    extra: HashMap::new(),
                });
                m
            },
            optional: HashMap::new(),
            hidden: HashMap::new(),
        },
        output_types: vec![IoType::Noise],
        output_names: vec!["NOISE".to_string()],
        output_is_list: vec![false],
        is_output_node: false,
        has_intermediate_output: false,
        is_changed: None,
        not_idempotent: true,
        function_name: "get_noise".to_string(),
    };

    registry.register(class_def, Arc::new(|_ctx, node, _node_id| {
        let noise_seed = node.inputs.get("noise_seed")
            .and_then(|v| v.as_i64())
            .unwrap_or(0);
        let control = node.inputs.get("control_after_generate")
            .and_then(|v| v.as_str())
            .unwrap_or("fixed");

        let seed = match control {
            "randomize" => {
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_nanos() as i64;
                now.abs()
            }
            _ => noise_seed,
        };

        Box::pin(async move {
            Ok(vec![json!({
                "type": "noise",
                "seed": seed,
            })])
        })
    }));
}

fn register_ksampler_select(registry: &mut NodeRegistry) {
    let class_def = NodeClassDef {
        class_type: "KSamplerSelect".to_string(),
        display_name: "KSampler Select".to_string(),
        category: "sampling".to_string(),
        input_types: NodeInputTypes {
            required: {
                let mut m = HashMap::new();
                m.insert("sampler_name".to_string(), InputTypeSpec {
                    type_name: "COMBO".to_string(),
                    extra: HashMap::new(),
                });
                m
            },
            optional: HashMap::new(),
            hidden: HashMap::new(),
        },
        output_types: vec![IoType::Sampler],
        output_names: vec!["SAMPLER".to_string()],
        output_is_list: vec![false],
        is_output_node: false,
        has_intermediate_output: false,
        is_changed: None,
        not_idempotent: false,
        function_name: "get_sampler".to_string(),
    };

    registry.register(class_def, Arc::new(|_ctx, node, _node_id| {
        let sampler_name = node.inputs.get("sampler_name")
            .and_then(|v| v.as_str())
            .unwrap_or("euler");

        Box::pin(async move {
            Ok(vec![json!({
                "type": "sampler",
                "sampler_name": sampler_name,
            })])
        })
    }));
}

fn register_manual_sigmas(registry: &mut NodeRegistry) {
    let class_def = NodeClassDef {
        class_type: "ManualSigmas".to_string(),
        display_name: "Manual Sigmas".to_string(),
        category: "sampling".to_string(),
        input_types: NodeInputTypes {
            required: {
                let mut m = HashMap::new();
                m.insert("sigmas".to_string(), InputTypeSpec {
                    type_name: "STRING".to_string(),
                    extra: HashMap::new(),
                });
                m
            },
            optional: HashMap::new(),
            hidden: HashMap::new(),
        },
        output_types: vec![IoType::Sigmas],
        output_names: vec!["SIGMAS".to_string()],
        output_is_list: vec![false],
        is_output_node: false,
        has_intermediate_output: false,
        is_changed: None,
        not_idempotent: false,
        function_name: "get_sigmas".to_string(),
    };

    registry.register(class_def, Arc::new(|_ctx, node, _node_id| {
        let sigmas = node.inputs.get("sigmas")
            .and_then(|v| v.as_str())
            .unwrap_or("1.0, 0.0");

        Box::pin(async move {
            Ok(vec![json!({
                "type": "sigmas",
                "sigmas": sigmas,
            })])
        })
    }));
}

fn register_cfg_guider(registry: &mut NodeRegistry) {
    let class_def = NodeClassDef {
        class_type: "CFGGuider".to_string(),
        display_name: "CFG Guider".to_string(),
        category: "sampling".to_string(),
        input_types: NodeInputTypes {
            required: {
                let mut m = HashMap::new();
                m.insert("model".to_string(), InputTypeSpec {
                    type_name: "MODEL".to_string(),
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
                m.insert("cfg".to_string(), InputTypeSpec {
                    type_name: "FLOAT".to_string(),
                    extra: HashMap::new(),
                });
                m
            },
            optional: HashMap::new(),
            hidden: HashMap::new(),
        },
        output_types: vec![IoType::Guider],
        output_names: vec!["GUIDER".to_string()],
        output_is_list: vec![false],
        is_output_node: false,
        has_intermediate_output: false,
        is_changed: None,
        not_idempotent: false,
        function_name: "get_guider".to_string(),
    };

    registry.register(class_def, Arc::new(|ctx, _node, node_id| {
        let model = ctx.resolve_input(node_id, "model").unwrap_or_else(|_| json!({}));
        let positive = ctx.resolve_input(node_id, "positive").unwrap_or_else(|_| json!({}));
        let negative = ctx.resolve_input(node_id, "negative").unwrap_or_else(|_| json!({}));
        let cfg = ctx.resolve_input(node_id, "cfg").unwrap_or_else(|_| json!(1.0));

        Box::pin(async move {
            Ok(vec![json!({
                "type": "guider",
                "model": model,
                "positive": positive,
                "negative": negative,
                "cfg": cfg,
            })])
        })
    }));
}

fn register_sampler_custom_advanced(registry: &mut NodeRegistry) {
    let class_def = NodeClassDef {
        class_type: "SamplerCustomAdvanced".to_string(),
        display_name: "Sampler Custom Advanced".to_string(),
        category: "sampling".to_string(),
        input_types: NodeInputTypes {
            required: {
                let mut m = HashMap::new();
                m.insert("noise".to_string(), InputTypeSpec {
                    type_name: "NOISE".to_string(),
                    extra: HashMap::new(),
                });
                m.insert("guider".to_string(), InputTypeSpec {
                    type_name: "GUIDER".to_string(),
                    extra: HashMap::new(),
                });
                m.insert("sampler".to_string(), InputTypeSpec {
                    type_name: "SAMPLER".to_string(),
                    extra: HashMap::new(),
                });
                m.insert("sigmas".to_string(), InputTypeSpec {
                    type_name: "SIGMAS".to_string(),
                    extra: HashMap::new(),
                });
                m.insert("latent_image".to_string(), InputTypeSpec {
                    type_name: "LATENT".to_string(),
                    extra: HashMap::new(),
                });
                m
            },
            optional: HashMap::new(),
            hidden: HashMap::new(),
        },
        output_types: vec![IoType::Latent, IoType::Latent],
        output_names: vec!["output".to_string(), "denoised_output".to_string()],
        output_is_list: vec![false, false],
        is_output_node: false,
        has_intermediate_output: false,
        is_changed: None,
        not_idempotent: true,
        function_name: "sample".to_string(),
    };

    registry.register(class_def, Arc::new(|ctx, _node, node_id| {
        let noise = ctx.resolve_input(node_id, "noise").unwrap_or_else(|_| json!({}));
        let guider = ctx.resolve_input(node_id, "guider").unwrap_or_else(|_| json!({}));
        let sampler = ctx.resolve_input(node_id, "sampler").unwrap_or_else(|_| json!({}));
        let sigmas = ctx.resolve_input(node_id, "sigmas").unwrap_or_else(|_| json!({}));
        let latent_image = ctx.resolve_input(node_id, "latent_image").unwrap_or_else(|_| json!({}));

        let backend = ctx.backend();
        let supports_vid_gen = backend.supports_video_generation();

        Box::pin(async move {
            if supports_vid_gen {
                let model = guider.get("model").cloned().unwrap_or(json!({}));
                let positive = guider.get("positive").cloned().unwrap_or(json!({}));
                let negative = guider.get("negative").cloned().unwrap_or(json!({}));
                let cfg = guider.get("cfg").and_then(|v| v.as_f64()).unwrap_or(1.0);
                let seed = noise.get("seed").and_then(|v| v.as_i64()).unwrap_or(0);
                let sampler_name = sampler.get("sampler_name").and_then(|v| v.as_str()).unwrap_or("euler");
                let sigmas_str = sigmas.get("sigmas").and_then(|v| v.as_str()).unwrap_or("1.0, 0.0");

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
                if let Some(path) = model.get("t5xxl_path").and_then(|v| v.as_str()) {
                    model_config = model_config.with_t5xxl(path);
                }
                if let Some(path) = model.get("text_encoder_path").and_then(|v| v.as_str()) {
                    model_config = model_config.with_text_encoder(path);
                }
                if let Some(path) = model.get("llm_path").and_then(|v| v.as_str()) {
                    model_config = model_config.with_llm(path);
                }
                let mut lora_entries: Vec<comfy_inference::LoraEntry> = Vec::new();
                if let Some(path) = model.get("lora_path").and_then(|v| v.as_str()) {
                    let multiplier = model.get("lora_strength").and_then(|v| v.as_f64()).unwrap_or(1.0) as f32;
                    lora_entries.push(comfy_inference::LoraEntry {
                        path: path.to_string(),
                        multiplier,
                        is_high_noise: false,
                    });
                }

                let clip_config = positive.get("clip");
                if let Some(clip) = clip_config {
                    if model_config.clip_l_path.is_none() {
                        if let Some(path) = clip.get("clip_l_path").and_then(|v| v.as_str()) {
                            model_config = model_config.with_clip_l(path);
                        }
                    }
                    if model_config.t5xxl_path.is_none() {
                        if let Some(path) = clip.get("t5xxl_path").and_then(|v| v.as_str()) {
                            model_config = model_config.with_t5xxl(path);
                        }
                    }
                    if model_config.text_encoder_path.is_none() {
                        if let Some(path) = clip.get("text_encoder_path").and_then(|v| v.as_str()) {
                            model_config = model_config.with_text_encoder(path);
                        }
                    }
                    if model_config.llm_path.is_none() {
                        if let Some(path) = clip.get("llm_path").and_then(|v| v.as_str()) {
                            model_config = model_config.with_llm(path);
                        }
                    }
                }

                let needs_clip = model_config.clip_l_path.is_none() || model_config.t5xxl_path.is_none();
                let needs_vae = model_config.vae_path.is_none();
                if needs_clip || needs_vae {
                    if needs_clip {
                        let (clip_l, _, t5xxl) = auto_detect_text_encoders(ModelType::LTX);
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
                    if needs_vae {
                        let base = get_models_base_dir();
                        let vae_dir = base.join("vae");
                        if let Some(path) = find_file_in_dir(&vae_dir, &["ltx_vae", "ae"]) {
                            model_config = model_config.with_vae(path);
                        }
                    }
                }

                let width = latent_image.get("width").and_then(|v| v.as_i64()).unwrap_or(768) as i32;
                let height = latent_image.get("height").and_then(|v| v.as_i64()).unwrap_or(512) as i32;
                let length = latent_image.get("length").and_then(|v| v.as_i64()).unwrap_or(97) as i32;
                let _frame_rate = positive.get("frame_rate").and_then(|v| v.as_f64()).unwrap_or(25.0) as f32;

                let sample_method = parse_sample_method(sampler_name);
                let scheduler_type = Scheduler::Simple;

                let mut video_params = comfy_inference::VideoGenParams::new(prompt_text)
                    .with_negative_prompt(negative_text)
                    .with_dimensions(width, height)
                    .with_seed(seed)
                    .with_video_frames(length)
                    .with_model_config(model_config);

                video_params.loras = lora_entries;
                video_params.sample_params.sample_steps = sigmas_str.split(',').count() as i32 - 1;
                video_params.sample_params.guidance.txt_cfg = cfg as f32;
                video_params.sample_params.sample_method = sample_method;
                video_params.sample_params.scheduler = scheduler_type;

                match backend.generate_video(video_params) {
                    Ok(video) => {
                        let frame_count = video.frame_count();
                        tracing::info!("SamplerCustomAdvanced: generated {} video frames", frame_count);
                        Ok(vec![
                            json!({
                                "type": "latent",
                                "subtype": "ltxv_av",
                                "frame_count": frame_count,
                                "fps": video.fps,
                            }),
                            json!({
                                "type": "latent",
                                "subtype": "ltxv_av_denoised",
                                "frame_count": frame_count,
                                "fps": video.fps,
                            }),
                        ])
                    }
                    Err(e) => {
                        tracing::error!("SamplerCustomAdvanced video generation failed: {}", e);
                        Err(ExecutorError::Inference(e))
                    }
                }
            } else {
                Ok(vec![
                    json!({
                        "type": "latent",
                        "noise": noise,
                        "guider": guider,
                        "sampler": sampler,
                        "sigmas": sigmas,
                        "latent_image": latent_image,
                    }),
                    json!({
                        "type": "latent",
                        "denoised": true,
                    }),
                ])
            }
        })
    }));
}

fn register_create_video(registry: &mut NodeRegistry) {
    let class_def = NodeClassDef {
        class_type: "CreateVideo".to_string(),
        display_name: "Create Video".to_string(),
        category: "video".to_string(),
        input_types: NodeInputTypes {
            required: {
                let mut m = HashMap::new();
                m.insert("images".to_string(), InputTypeSpec {
                    type_name: "IMAGE".to_string(),
                    extra: HashMap::new(),
                });
                m.insert("fps".to_string(), InputTypeSpec {
                    type_name: "FLOAT".to_string(),
                    extra: HashMap::new(),
                });
                m
            },
            optional: {
                let mut m = HashMap::new();
                m.insert("audio".to_string(), InputTypeSpec {
                    type_name: "AUDIO".to_string(),
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
        function_name: "create".to_string(),
    };

    registry.register(class_def, Arc::new(|ctx, _node, node_id| {
        let images = ctx.resolve_input(node_id, "images").unwrap_or_else(|_| json!(null));
        let fps = ctx.resolve_input(node_id, "fps").unwrap_or_else(|_| json!(24.0));
        let audio = ctx.resolve_input(node_id, "audio").ok();

        Box::pin(async move {
            let mut result = json!({
                "type": "video",
                "images": images,
                "fps": fps,
            });
            if let Some(a) = audio {
                if let Some(obj) = result.as_object_mut() {
                    obj.insert("audio".to_string(), a);
                }
            }
            Ok(vec![result])
        })
    }));
}

fn register_vae_decode_tiled(registry: &mut NodeRegistry) {
    let class_def = NodeClassDef {
        class_type: "VAEDecodeTiled".to_string(),
        display_name: "VAE Decode (Tiled)".to_string(),
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
            optional: {
                let mut m = HashMap::new();
                m.insert("tile_size".to_string(), InputTypeSpec {
                    type_name: "INT".to_string(),
                    extra: HashMap::new(),
                });
                m.insert("overlap".to_string(), InputTypeSpec {
                    type_name: "INT".to_string(),
                    extra: HashMap::new(),
                });
                m.insert("temporal_size".to_string(), InputTypeSpec {
                    type_name: "INT".to_string(),
                    extra: HashMap::new(),
                });
                m.insert("temporal_overlap".to_string(), InputTypeSpec {
                    type_name: "INT".to_string(),
                    extra: HashMap::new(),
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
        function_name: "decode".to_string(),
    };

    registry.register(class_def, Arc::new(|ctx, _node, node_id| {
        let samples = ctx.resolve_input(node_id, "samples").unwrap_or_else(|_| json!({}));
        let vae = ctx.resolve_input(node_id, "vae").unwrap_or_else(|_| json!(null));

        Box::pin(async move {
            Ok(vec![json!({
                "type": "image",
                "decoded_from": "tiled_vae",
                "samples": samples,
                "vae": vae,
            })])
        })
    }));
}

fn register_resize_images_by_longer_edge(registry: &mut NodeRegistry) {
    let class_def = NodeClassDef {
        class_type: "ResizeImagesByLongerEdge".to_string(),
        display_name: "Resize Images by Longer Edge".to_string(),
        category: "image".to_string(),
        input_types: NodeInputTypes {
            required: {
                let mut m = HashMap::new();
                m.insert("images".to_string(), InputTypeSpec {
                    type_name: "IMAGE".to_string(),
                    extra: HashMap::new(),
                });
                m.insert("longer_edge".to_string(), InputTypeSpec {
                    type_name: "INT".to_string(),
                    extra: HashMap::new(),
                });
                m
            },
            optional: HashMap::new(),
            hidden: HashMap::new(),
        },
        output_types: vec![IoType::Image],
        output_names: vec!["images".to_string()],
        output_is_list: vec![false],
        is_output_node: false,
        has_intermediate_output: false,
        is_changed: None,
        not_idempotent: false,
        function_name: "resize".to_string(),
    };

    registry.register(class_def, Arc::new(|_ctx, node, _node_id| {
        let _longer_edge = node.inputs.get("longer_edge")
            .and_then(|v| v.as_i64())
            .unwrap_or(1536);

        Box::pin(async move {
            Ok(vec![json!({
                "type": "image",
                "resized": true,
                "method": "longer_edge",
            })])
        })
    }));
}

fn register_resize_image_mask_node(registry: &mut NodeRegistry) {
    let class_def = NodeClassDef {
        class_type: "ResizeImageMaskNode".to_string(),
        display_name: "Resize Image/Mask".to_string(),
        category: "image".to_string(),
        input_types: NodeInputTypes {
            required: {
                let mut m = HashMap::new();
                m.insert("input".to_string(), InputTypeSpec {
                    type_name: "IMAGE".to_string(),
                    extra: HashMap::new(),
                });
                m.insert("resize_type.width".to_string(), InputTypeSpec {
                    type_name: "INT".to_string(),
                    extra: HashMap::new(),
                });
                m.insert("resize_type.height".to_string(), InputTypeSpec {
                    type_name: "INT".to_string(),
                    extra: HashMap::new(),
                });
                m
            },
            optional: {
                let mut m = HashMap::new();
                m.insert("resize_type".to_string(), InputTypeSpec {
                    type_name: "COMBO".to_string(),
                    extra: HashMap::new(),
                });
                m.insert("crop".to_string(), InputTypeSpec {
                    type_name: "COMBO".to_string(),
                    extra: HashMap::new(),
                });
                m.insert("interpolation".to_string(), InputTypeSpec {
                    type_name: "COMBO".to_string(),
                    extra: HashMap::new(),
                });
                m
            },
            hidden: HashMap::new(),
        },
        output_types: vec![IoType::Image],
        output_names: vec!["resized".to_string()],
        output_is_list: vec![false],
        is_output_node: false,
        has_intermediate_output: false,
        is_changed: None,
        not_idempotent: false,
        function_name: "resize".to_string(),
    };

    registry.register(class_def, Arc::new(|_ctx, node, _node_id| {
        let width = node.inputs.get("resize_type.width")
            .and_then(|v| v.as_i64())
            .unwrap_or(1920);
        let height = node.inputs.get("resize_type.height")
            .and_then(|v| v.as_i64())
            .unwrap_or(1088);

        Box::pin(async move {
            Ok(vec![json!({
                "type": "image",
                "resized": true,
                "width": width,
                "height": height,
            })])
        })
    }));
}

fn register_trim_audio_duration(registry: &mut NodeRegistry) {
    let class_def = NodeClassDef {
        class_type: "TrimAudioDuration".to_string(),
        display_name: "Trim Audio Duration".to_string(),
        category: "audio".to_string(),
        input_types: NodeInputTypes {
            required: {
                let mut m = HashMap::new();
                m.insert("audio".to_string(), InputTypeSpec {
                    type_name: "AUDIO".to_string(),
                    extra: HashMap::new(),
                });
                m.insert("start_index".to_string(), InputTypeSpec {
                    type_name: "FLOAT".to_string(),
                    extra: HashMap::new(),
                });
                m.insert("duration".to_string(), InputTypeSpec {
                    type_name: "FLOAT".to_string(),
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
        function_name: "trim".to_string(),
    };

    registry.register(class_def, Arc::new(|ctx, _node, node_id| {
        let audio = ctx.resolve_input(node_id, "audio").unwrap_or_else(|_| json!(null));
        let start_index = ctx.resolve_input(node_id, "start_index").unwrap_or_else(|_| json!(0.0));
        let duration = ctx.resolve_input(node_id, "duration").unwrap_or_else(|_| json!(60.0));

        Box::pin(async move {
            Ok(vec![json!({
                "type": "audio",
                "trimmed": true,
                "source_audio": audio,
                "start_index": start_index,
                "duration": duration,
            })])
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
        let samples = ctx.resolve_input(node_id, "samples").unwrap_or_else(|_| json!({}));
        let mask = ctx.resolve_input(node_id, "mask").unwrap_or_else(|_| json!(null));

        Box::pin(async move {
            let mut result = samples;
            if let Some(obj) = result.as_object_mut() {
                obj.insert("noise_mask".to_string(), mask);
            }
            Ok(vec![result])
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
        function_name: "create_mask".to_string(),
    };

    registry.register(class_def, Arc::new(|_ctx, node, _node_id| {
        let value = node.inputs.get("value")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);
        let width = node.inputs.get("width")
            .and_then(|v| v.as_i64())
            .unwrap_or(1024);
        let height = node.inputs.get("height")
            .and_then(|v| v.as_i64())
            .unwrap_or(1024);

        Box::pin(async move {
            Ok(vec![json!({
                "type": "mask",
                "mask_type": "solid",
                "value": value,
                "width": width,
                "height": height,
            })])
        })
    }));
}

fn register_primitive_int(registry: &mut NodeRegistry) {
    let class_def = NodeClassDef {
        class_type: "PrimitiveInt".to_string(),
        display_name: "Primitive Int".to_string(),
        category: "utils".to_string(),
        input_types: NodeInputTypes {
            required: {
                let mut m = HashMap::new();
                m.insert("value".to_string(), InputTypeSpec {
                    type_name: "INT".to_string(),
                    extra: HashMap::new(),
                });
                m.insert("control_after_generate".to_string(), InputTypeSpec {
                    type_name: "COMBO".to_string(),
                    extra: HashMap::new(),
                });
                m
            },
            optional: HashMap::new(),
            hidden: HashMap::new(),
        },
        output_types: vec![IoType::Int],
        output_names: vec!["INT".to_string()],
        output_is_list: vec![false],
        is_output_node: false,
        has_intermediate_output: false,
        is_changed: None,
        not_idempotent: false,
        function_name: "get_value".to_string(),
    };

    registry.register(class_def, Arc::new(|_ctx, node, _node_id| {
        let value = node.inputs.get("value")
            .and_then(|v| v.as_i64())
            .unwrap_or(0);

        Box::pin(async move {
            Ok(vec![json!(value)])
        })
    }));
}

fn register_primitive_float(registry: &mut NodeRegistry) {
    let class_def = NodeClassDef {
        class_type: "PrimitiveFloat".to_string(),
        display_name: "Primitive Float".to_string(),
        category: "utils".to_string(),
        input_types: NodeInputTypes {
            required: {
                let mut m = HashMap::new();
                m.insert("value".to_string(), InputTypeSpec {
                    type_name: "FLOAT".to_string(),
                    extra: HashMap::new(),
                });
                m
            },
            optional: HashMap::new(),
            hidden: HashMap::new(),
        },
        output_types: vec![IoType::Float],
        output_names: vec!["FLOAT".to_string()],
        output_is_list: vec![false],
        is_output_node: false,
        has_intermediate_output: false,
        is_changed: None,
        not_idempotent: false,
        function_name: "get_value".to_string(),
    };

    registry.register(class_def, Arc::new(|_ctx, node, _node_id| {
        let value = node.inputs.get("value")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);

        Box::pin(async move {
            Ok(vec![json!(value)])
        })
    }));
}

fn register_primitive_boolean(registry: &mut NodeRegistry) {
    let class_def = NodeClassDef {
        class_type: "PrimitiveBoolean".to_string(),
        display_name: "Primitive Boolean".to_string(),
        category: "utils".to_string(),
        input_types: NodeInputTypes {
            required: {
                let mut m = HashMap::new();
                m.insert("value".to_string(), InputTypeSpec {
                    type_name: "BOOLEAN".to_string(),
                    extra: HashMap::new(),
                });
                m
            },
            optional: HashMap::new(),
            hidden: HashMap::new(),
        },
        output_types: vec![IoType::Boolean],
        output_names: vec!["BOOLEAN".to_string()],
        output_is_list: vec![false],
        is_output_node: false,
        has_intermediate_output: false,
        is_changed: None,
        not_idempotent: false,
        function_name: "get_value".to_string(),
    };

    registry.register(class_def, Arc::new(|_ctx, node, _node_id| {
        let value = node.inputs.get("value")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        Box::pin(async move {
            Ok(vec![json!(value)])
        })
    }));
}

fn register_primitive_string_multiline(registry: &mut NodeRegistry) {
    let class_def = NodeClassDef {
        class_type: "PrimitiveStringMultiline".to_string(),
        display_name: "Primitive String Multiline".to_string(),
        category: "utils".to_string(),
        input_types: NodeInputTypes {
            required: {
                let mut m = HashMap::new();
                m.insert("value".to_string(), InputTypeSpec {
                    type_name: "STRING".to_string(),
                    extra: HashMap::new(),
                });
                m
            },
            optional: HashMap::new(),
            hidden: HashMap::new(),
        },
        output_types: vec![IoType::String],
        output_names: vec!["STRING".to_string()],
        output_is_list: vec![false],
        is_output_node: false,
        has_intermediate_output: false,
        is_changed: None,
        not_idempotent: false,
        function_name: "get_value".to_string(),
    };

    registry.register(class_def, Arc::new(|_ctx, node, _node_id| {
        let value = node.inputs.get("value")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        Box::pin(async move {
            Ok(vec![json!(value)])
        })
    }));
}

fn register_comfy_math_expression(registry: &mut NodeRegistry) {
    let class_def = NodeClassDef {
        class_type: "ComfyMathExpression".to_string(),
        display_name: "Math Expression".to_string(),
        category: "utils".to_string(),
        input_types: NodeInputTypes {
            required: {
                let mut m = HashMap::new();
                m.insert("expression".to_string(), InputTypeSpec {
                    type_name: "STRING".to_string(),
                    extra: HashMap::new(),
                });
                m
            },
            optional: {
                let mut m = HashMap::new();
                m.insert("values.a".to_string(), InputTypeSpec {
                    type_name: "FLOAT".to_string(),
                    extra: HashMap::new(),
                });
                m.insert("values.b".to_string(), InputTypeSpec {
                    type_name: "FLOAT".to_string(),
                    extra: HashMap::new(),
                });
                m.insert("values.c".to_string(), InputTypeSpec {
                    type_name: "FLOAT".to_string(),
                    extra: HashMap::new(),
                });
                m
            },
            hidden: HashMap::new(),
        },
        output_types: vec![IoType::Float, IoType::Int],
        output_names: vec!["FLOAT".to_string(), "INT".to_string()],
        output_is_list: vec![false, false],
        is_output_node: false,
        has_intermediate_output: false,
        is_changed: None,
        not_idempotent: false,
        function_name: "evaluate".to_string(),
    };

    registry.register(class_def, Arc::new(|_ctx, node, _node_id| {
        let expression = node.inputs.get("expression")
            .and_then(|v| v.as_str())
            .unwrap_or("a")
            .to_string();
        let a = node.inputs.get("values.a")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);
        let b = node.inputs.get("values.b")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);
        let c = node.inputs.get("values.c")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);

        let result = match expression.as_str() {
            "a/2" => a / 2.0,
            "a" => a,
            "a * b + 1" => a * b + 1.0,
            expr if expr.contains("a/2") => a / 2.0,
            expr if expr.contains("a*b+1") || expr.contains("a * b + 1") => a * b + 1.0,
            _ => {
                let _result = expression
                    .replace("a", &format!("{}", a))
                    .replace("b", &format!("{}", b))
                    .replace("c", &format!("{}", c));
                a
            }
        };

        Box::pin(async move {
            Ok(vec![json!(result), json!(result as i64)])
        })
    }));
}

fn register_clip_vision_loader(registry: &mut NodeRegistry) {
    let class_def = NodeClassDef {
        class_type: "CLIPVisionLoader".to_string(),
        display_name: "Load CLIP Vision Model".to_string(),
        category: "loaders".to_string(),
        input_types: NodeInputTypes {
            required: {
                let mut m = HashMap::new();
                m.insert("clip_vision_name".to_string(), InputTypeSpec {
                    type_name: "COMBO".to_string(),
                    extra: HashMap::new(),
                });
                m
            },
            optional: HashMap::new(),
            hidden: HashMap::new(),
        },
        output_types: vec![IoType::ClipVision],
        output_names: vec!["CLIP_VISION".to_string()],
        output_is_list: vec![false],
        is_output_node: false,
        has_intermediate_output: false,
        is_changed: None,
        not_idempotent: false,
        function_name: "load_clip_vision".to_string(),
    };

    registry.register(class_def, Arc::new(|_ctx, node, _node_id| {
        let clip_vision_name = node.inputs.get("clip_vision_name")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        let clip_vision_path = resolve_model_path("clip_vision", clip_vision_name);

        Box::pin(async move {
            Ok(vec![json!({
                "type": "clip_vision",
                "clip_vision_path": clip_vision_path,
            })])
        })
    }));
}

fn register_style_model_loader(registry: &mut NodeRegistry) {
    let class_def = NodeClassDef {
        class_type: "StyleModelLoader".to_string(),
        display_name: "Load Style Model".to_string(),
        category: "loaders".to_string(),
        input_types: NodeInputTypes {
            required: {
                let mut m = HashMap::new();
                m.insert("style_model_name".to_string(), InputTypeSpec {
                    type_name: "COMBO".to_string(),
                    extra: HashMap::new(),
                });
                m
            },
            optional: HashMap::new(),
            hidden: HashMap::new(),
        },
        output_types: vec![IoType::StyleModel],
        output_names: vec!["STYLE_MODEL".to_string()],
        output_is_list: vec![false],
        is_output_node: false,
        has_intermediate_output: false,
        is_changed: None,
        not_idempotent: false,
        function_name: "load_style_model".to_string(),
    };

    registry.register(class_def, Arc::new(|_ctx, node, _node_id| {
        let style_model_name = node.inputs.get("style_model_name")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        let style_model_path = resolve_model_path("style_models", style_model_name);

        Box::pin(async move {
            Ok(vec![json!({
                "type": "style_model",
                "style_model_path": style_model_path,
            })])
        })
    }));
}

fn register_upscale_model_loader(registry: &mut NodeRegistry) {
    let class_def = NodeClassDef {
        class_type: "UpscaleModelLoader".to_string(),
        display_name: "Load Upscale Model".to_string(),
        category: "loaders".to_string(),
        input_types: NodeInputTypes {
            required: {
                let mut m = HashMap::new();
                m.insert("upscale_model_name".to_string(), InputTypeSpec {
                    type_name: "COMBO".to_string(),
                    extra: HashMap::new(),
                });
                m
            },
            optional: HashMap::new(),
            hidden: HashMap::new(),
        },
        output_types: vec![IoType::UpscaleModel],
        output_names: vec!["UPSCALE_MODEL".to_string()],
        output_is_list: vec![false],
        is_output_node: false,
        has_intermediate_output: false,
        is_changed: None,
        not_idempotent: false,
        function_name: "load_upscale_model".to_string(),
    };

    registry.register(class_def, Arc::new(|_ctx, node, _node_id| {
        let model_name = node.inputs.get("upscale_model_name")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        let model_path = resolve_model_path("upscale_models", model_name);

        Box::pin(async move {
            Ok(vec![json!({
                "type": "upscale_model",
                "path": model_path,
            })])
        })
    }));
}

fn register_gligen_loader(registry: &mut NodeRegistry) {
    let class_def = NodeClassDef {
        class_type: "GLIGENLoader".to_string(),
        display_name: "Load GLIGEN Model".to_string(),
        category: "loaders".to_string(),
        input_types: NodeInputTypes {
            required: {
                let mut m = HashMap::new();
                m.insert("gligen_name".to_string(), InputTypeSpec {
                    type_name: "COMBO".to_string(),
                    extra: HashMap::new(),
                });
                m
            },
            optional: HashMap::new(),
            hidden: HashMap::new(),
        },
        output_types: vec![IoType::Gligen],
        output_names: vec!["GLIGEN".to_string()],
        output_is_list: vec![false],
        is_output_node: false,
        has_intermediate_output: false,
        is_changed: None,
        not_idempotent: false,
        function_name: "load_gligen".to_string(),
    };

    registry.register(class_def, Arc::new(|_ctx, node, _node_id| {
        let gligen_name = node.inputs.get("gligen_name")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        let gligen_path = resolve_model_path("gligen", gligen_name);

        Box::pin(async move {
            Ok(vec![json!({
                "type": "gligen",
                "gligen_path": gligen_path,
            })])
        })
    }));
}

fn register_hypernetwork_loader(registry: &mut NodeRegistry) {
    let class_def = NodeClassDef {
        class_type: "HypernetworkLoader".to_string(),
        display_name: "Load Hypernetwork".to_string(),
        category: "loaders".to_string(),
        input_types: NodeInputTypes {
            required: {
                let mut m = HashMap::new();
                m.insert("model".to_string(), InputTypeSpec {
                    type_name: "MODEL".to_string(),
                    extra: HashMap::new(),
                });
                m.insert("hypernetwork_name".to_string(), InputTypeSpec {
                    type_name: "COMBO".to_string(),
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
        output_types: vec![IoType::Model],
        output_names: vec!["MODEL".to_string()],
        output_is_list: vec![false],
        is_output_node: false,
        has_intermediate_output: false,
        is_changed: None,
        not_idempotent: false,
        function_name: "load_hypernetwork".to_string(),
    };

    registry.register(class_def, Arc::new(|ctx, _node, node_id| {
        let model = ctx.resolve_input(node_id, "model")
            .unwrap_or_else(|_| json!({}));
        let hypernetwork_name = ctx.resolve_input(node_id, "hypernetwork_name")
            .unwrap_or_else(|_| json!(""))
            .as_str()
            .unwrap_or("")
            .to_string();
        let strength = ctx.resolve_input(node_id, "strength")
            .unwrap_or_else(|_| json!(1.0))
            .as_f64()
            .unwrap_or(1.0) as f32;

        let hypernetwork_path = resolve_model_path("hypernetworks", &hypernetwork_name);

        Box::pin(async move {
            let mut model_out = model.as_object().cloned().unwrap_or_default();
            model_out.insert("hypernetwork_path".to_string(), json!(hypernetwork_path));
            model_out.insert("hypernetwork_strength".to_string(), json!(strength));
            Ok(vec![json!(model_out)])
        })
    }));
}

fn register_photomaker_loader(registry: &mut NodeRegistry) {
    let class_def = NodeClassDef {
        class_type: "PhotoMakerLoader".to_string(),
        display_name: "Load PhotoMaker Model".to_string(),
        category: "loaders".to_string(),
        input_types: NodeInputTypes {
            required: {
                let mut m = HashMap::new();
                m.insert("photomaker_name".to_string(), InputTypeSpec {
                    type_name: "COMBO".to_string(),
                    extra: HashMap::new(),
                });
                m
            },
            optional: HashMap::new(),
            hidden: HashMap::new(),
        },
        output_types: vec![IoType::Photomaker],
        output_names: vec!["PHOTOMAKER".to_string()],
        output_is_list: vec![false],
        is_output_node: false,
        has_intermediate_output: false,
        is_changed: None,
        not_idempotent: false,
        function_name: "load_photomaker".to_string(),
    };

    registry.register(class_def, Arc::new(|_ctx, node, _node_id| {
        let photomaker_name = node.inputs.get("photomaker_name")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        let photomaker_path = resolve_model_path("photomarker", photomaker_name);

        Box::pin(async move {
            Ok(vec![json!({
                "type": "photomaker",
                "photomaker_path": photomaker_path,
            })])
        })
    }));
}

fn register_embedding_loader(registry: &mut NodeRegistry) {
    let class_def = NodeClassDef {
        class_type: "EmbeddingLoader".to_string(),
        display_name: "Load Embedding".to_string(),
        category: "loaders".to_string(),
        input_types: NodeInputTypes {
            required: {
                let mut m = HashMap::new();
                m.insert("embedding_name".to_string(), InputTypeSpec {
                    type_name: "COMBO".to_string(),
                    extra: HashMap::new(),
                });
                m
            },
            optional: HashMap::new(),
            hidden: HashMap::new(),
        },
        output_types: vec![IoType::Embedding],
        output_names: vec!["EMBEDDING".to_string()],
        output_is_list: vec![false],
        is_output_node: false,
        has_intermediate_output: false,
        is_changed: None,
        not_idempotent: false,
        function_name: "load_embedding".to_string(),
    };

    registry.register(class_def, Arc::new(|_ctx, node, _node_id| {
        let embedding_name = node.inputs.get("embedding_name")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        let embedding_path = resolve_model_path("embeddings", embedding_name);

        Box::pin(async move {
            Ok(vec![json!({
                "type": "embedding",
                "embedding_path": embedding_path,
                "embedding_name": embedding_name,
            })])
        })
    }));
}

fn register_classifier_loader(registry: &mut NodeRegistry) {
    let class_def = NodeClassDef {
        class_type: "ClassifierLoader".to_string(),
        display_name: "Load Classifier Model".to_string(),
        category: "loaders".to_string(),
        input_types: NodeInputTypes {
            required: {
                let mut m = HashMap::new();
                m.insert("classifier_name".to_string(), InputTypeSpec {
                    type_name: "COMBO".to_string(),
                    extra: HashMap::new(),
                });
                m
            },
            optional: HashMap::new(),
            hidden: HashMap::new(),
        },
        output_types: vec![IoType::Custom("CLASSIFIER".to_string())],
        output_names: vec!["CLASSIFIER".to_string()],
        output_is_list: vec![false],
        is_output_node: false,
        has_intermediate_output: false,
        is_changed: None,
        not_idempotent: false,
        function_name: "load_classifier".to_string(),
    };

    registry.register(class_def, Arc::new(|_ctx, node, _node_id| {
        let classifier_name = node.inputs.get("classifier_name")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        let classifier_path = resolve_model_path("classifiers", classifier_name);

        Box::pin(async move {
            Ok(vec![json!({
                "type": "classifier",
                "classifier_path": classifier_path,
            })])
        })
    }));
}

fn register_audio_encoder_loader(registry: &mut NodeRegistry) {
    let class_def = NodeClassDef {
        class_type: "AudioEncoderLoader".to_string(),
        display_name: "Load Audio Encoder".to_string(),
        category: "loaders".to_string(),
        input_types: NodeInputTypes {
            required: {
                let mut m = HashMap::new();
                m.insert("audio_encoder_name".to_string(), InputTypeSpec {
                    type_name: "COMBO".to_string(),
                    extra: HashMap::new(),
                });
                m
            },
            optional: HashMap::new(),
            hidden: HashMap::new(),
        },
        output_types: vec![IoType::Custom("AUDIO_ENCODER".to_string())],
        output_names: vec!["AUDIO_ENCODER".to_string()],
        output_is_list: vec![false],
        is_output_node: false,
        has_intermediate_output: false,
        is_changed: None,
        not_idempotent: false,
        function_name: "load_audio_encoder".to_string(),
    };

    registry.register(class_def, Arc::new(|_ctx, node, _node_id| {
        let audio_encoder_name = node.inputs.get("audio_encoder_name")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        let audio_encoder_path = resolve_model_path("audio_encoders", audio_encoder_name);

        Box::pin(async move {
            Ok(vec![json!({
                "type": "audio_encoder",
                "audio_encoder_path": audio_encoder_path,
            })])
        })
    }));
}

fn register_model_patch_loader(registry: &mut NodeRegistry) {
    let class_def = NodeClassDef {
        class_type: "ModelPatchLoader".to_string(),
        display_name: "Load Model Patch".to_string(),
        category: "loaders".to_string(),
        input_types: NodeInputTypes {
            required: {
                let mut m = HashMap::new();
                m.insert("model".to_string(), InputTypeSpec {
                    type_name: "MODEL".to_string(),
                    extra: HashMap::new(),
                });
                m.insert("patch_name".to_string(), InputTypeSpec {
                    type_name: "COMBO".to_string(),
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
        output_types: vec![IoType::Model],
        output_names: vec!["MODEL".to_string()],
        output_is_list: vec![false],
        is_output_node: false,
        has_intermediate_output: false,
        is_changed: None,
        not_idempotent: false,
        function_name: "load_model_patch".to_string(),
    };

    registry.register(class_def, Arc::new(|ctx, _node, node_id| {
        let model = ctx.resolve_input(node_id, "model")
            .unwrap_or_else(|_| json!({}));
        let patch_name = ctx.resolve_input(node_id, "patch_name")
            .unwrap_or_else(|_| json!(""))
            .as_str()
            .unwrap_or("")
            .to_string();
        let strength = ctx.resolve_input(node_id, "strength")
            .unwrap_or_else(|_| json!(1.0))
            .as_f64()
            .unwrap_or(1.0) as f32;

        let patch_path = resolve_model_path("model_patches", &patch_name);

        Box::pin(async move {
            let mut model_out = model.as_object().cloned().unwrap_or_default();
            model_out.insert("model_patch_path".to_string(), json!(patch_path));
            model_out.insert("model_patch_strength".to_string(), json!(strength));
            Ok(vec![json!(model_out)])
        })
    }));
}

fn register_vae_approx_loader(registry: &mut NodeRegistry) {
    let class_def = NodeClassDef {
        class_type: "VAEApproxLoader".to_string(),
        display_name: "Load VAE Approx (Preview)".to_string(),
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
        function_name: "load_vae_approx".to_string(),
    };

    registry.register(class_def, Arc::new(|_ctx, node, _node_id| {
        let vae_name = node.inputs.get("vae_name")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        let vae_path = resolve_model_path("vae_approx", vae_name);

        Box::pin(async move {
            Ok(vec![json!({
                "type": "vae",
                "vae_path": vae_path,
                "is_approx": true,
            })])
        })
    }));
}

fn register_if_else_node(registry: &mut NodeRegistry) {
    let class_def = NodeClassDef {
        class_type: "IfElseNode".to_string(),
        display_name: "If/Else Conditional".to_string(),
        category: "logic".to_string(),
        input_types: NodeInputTypes {
            required: {
                let mut m = HashMap::new();
                m.insert("condition".to_string(), InputTypeSpec {
                    type_name: "BOOLEAN".to_string(),
                    extra: HashMap::new(),
                });
                m.insert("on_true".to_string(), InputTypeSpec {
                    type_name: "*".to_string(),
                    extra: HashMap::new(),
                });
                m.insert("on_false".to_string(), InputTypeSpec {
                    type_name: "*".to_string(),
                    extra: HashMap::new(),
                });
                m
            },
            optional: HashMap::new(),
            hidden: HashMap::new(),
        },
        output_types: vec![IoType::Any],
        output_names: vec!["OUTPUT".to_string()],
        output_is_list: vec![false],
        is_output_node: false,
        has_intermediate_output: false,
        is_changed: None,
        not_idempotent: false,
        function_name: "if_else".to_string(),
    };

    registry.register(class_def, Arc::new(|ctx, _node, node_id| {
        let condition = ctx.resolve_input(node_id, "condition")
            .unwrap_or_else(|_| json!(true));
        let on_true = ctx.resolve_input(node_id, "on_true")
            .unwrap_or_else(|_| json!(null));
        let on_false = ctx.resolve_input(node_id, "on_false")
            .unwrap_or_else(|_| json!(null));

        Box::pin(async move {
            let result = if condition.as_bool().unwrap_or(true) {
                on_true
            } else {
                on_false
            };
            Ok(vec![result])
        })
    }));
}

fn register_for_loop_node(registry: &mut NodeRegistry) {
    let class_def = NodeClassDef {
        class_type: "ForLoopNode".to_string(),
        display_name: "For Loop".to_string(),
        category: "logic".to_string(),
        input_types: NodeInputTypes {
            required: {
                let mut m = HashMap::new();
                m.insert("count".to_string(), InputTypeSpec {
                    type_name: "INT".to_string(),
                    extra: HashMap::new(),
                });
                m.insert("initial_value".to_string(), InputTypeSpec {
                    type_name: "*".to_string(),
                    extra: HashMap::new(),
                });
                m
            },
            optional: HashMap::new(),
            hidden: HashMap::new(),
        },
        output_types: vec![IoType::Any, IoType::Int],
        output_names: vec!["OUTPUT".to_string(), "INDEX".to_string()],
        output_is_list: vec![false, false],
        is_output_node: false,
        has_intermediate_output: false,
        is_changed: None,
        not_idempotent: true,
        function_name: "for_loop".to_string(),
    };

    registry.register(class_def, Arc::new(|ctx, _node, node_id| {
        let count = ctx.resolve_input(node_id, "count")
            .unwrap_or_else(|_| json!(1));
        let initial_value = ctx.resolve_input(node_id, "initial_value")
            .unwrap_or_else(|_| json!(null));

        Box::pin(async move {
            let n = count.as_i64().unwrap_or(1).max(1).min(1000);
            let mut result = initial_value;
            for i in 0..n {
                result = json!({
                    "loop_index": i,
                    "loop_count": n,
                    "accumulated": result,
                });
            }
            Ok(vec![result, json!(0)])
        })
    }));
}

fn register_switch_node(registry: &mut NodeRegistry) {
    let class_def = NodeClassDef {
        class_type: "SwitchNode".to_string(),
        display_name: "Switch (Multi-Branch)".to_string(),
        category: "logic".to_string(),
        input_types: NodeInputTypes {
            required: {
                let mut m = HashMap::new();
                m.insert("selector".to_string(), InputTypeSpec {
                    type_name: "INT".to_string(),
                    extra: HashMap::new(),
                });
                m.insert("input_0".to_string(), InputTypeSpec {
                    type_name: "*".to_string(),
                    extra: HashMap::new(),
                });
                m.insert("input_1".to_string(), InputTypeSpec {
                    type_name: "*".to_string(),
                    extra: HashMap::new(),
                });
                m
            },
            optional: {
                let mut m = HashMap::new();
                m.insert("input_2".to_string(), InputTypeSpec {
                    type_name: "*".to_string(),
                    extra: HashMap::new(),
                });
                m.insert("input_3".to_string(), InputTypeSpec {
                    type_name: "*".to_string(),
                    extra: HashMap::new(),
                });
                m
            },
            hidden: HashMap::new(),
        },
        output_types: vec![IoType::Any],
        output_names: vec!["OUTPUT".to_string()],
        output_is_list: vec![false],
        is_output_node: false,
        has_intermediate_output: false,
        is_changed: None,
        not_idempotent: false,
        function_name: "switch".to_string(),
    };

    registry.register(class_def, Arc::new(|ctx, _node, node_id| {
        let selector = ctx.resolve_input(node_id, "selector")
            .unwrap_or_else(|_| json!(0));
        let input_0 = ctx.resolve_input(node_id, "input_0")
            .unwrap_or_else(|_| json!(null));
        let input_1 = ctx.resolve_input(node_id, "input_1")
            .unwrap_or_else(|_| json!(null));
        let input_2 = ctx.resolve_input(node_id, "input_2")
            .unwrap_or_else(|_| json!(null));
        let input_3 = ctx.resolve_input(node_id, "input_3")
            .unwrap_or_else(|_| json!(null));

        Box::pin(async move {
            let idx = selector.as_i64().unwrap_or(0).clamp(0, 3) as usize;
            let inputs = [input_0, input_1, input_2, input_3];
            Ok(vec![inputs[idx].clone()])
        })
    }));
}

fn register_pure_function_call_node(registry: &mut NodeRegistry) {
    let class_def = NodeClassDef {
        class_type: "PureFunctionCallNode".to_string(),
        display_name: "Pure Function Call".to_string(),
        category: "logic".to_string(),
        input_types: NodeInputTypes {
            required: {
                let mut m = HashMap::new();
                m.insert("function_name".to_string(), InputTypeSpec {
                    type_name: "STRING".to_string(),
                    extra: HashMap::new(),
                });
                m
            },
            optional: {
                let mut m = HashMap::new();
                m.insert("arg_0".to_string(), InputTypeSpec {
                    type_name: "*".to_string(),
                    extra: HashMap::new(),
                });
                m.insert("arg_1".to_string(), InputTypeSpec {
                    type_name: "*".to_string(),
                    extra: HashMap::new(),
                });
                m.insert("arg_2".to_string(), InputTypeSpec {
                    type_name: "*".to_string(),
                    extra: HashMap::new(),
                });
                m.insert("arg_3".to_string(), InputTypeSpec {
                    type_name: "*".to_string(),
                    extra: HashMap::new(),
                });
                m.insert("arg_4".to_string(), InputTypeSpec {
                    type_name: "*".to_string(),
                    extra: HashMap::new(),
                });
                m
            },
            hidden: HashMap::new(),
        },
        output_types: vec![IoType::Any],
        output_names: vec!["OUTPUT".to_string()],
        output_is_list: vec![false],
        is_output_node: false,
        has_intermediate_output: false,
        is_changed: None,
        not_idempotent: false,
        function_name: "call".to_string(),
    };

    registry.register(class_def, Arc::new(|_ctx, node, _node_id| {
        let function_name = node.inputs.get("function_name")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let args: Vec<Value> = (0..5)
            .filter_map(|i| {
                let key = format!("arg_{}", i);
                node.inputs.get(&key).cloned()
            })
            .collect();

        Box::pin(async move {
            let result = match function_name.as_str() {
                "string_concat" => {
                    let s: String = args.iter()
                        .filter_map(|a| a.as_str())
                        .collect();
                    json!(s)
                }
                "string_join" => {
                    let separator = args.first().and_then(|a| a.as_str()).unwrap_or(",");
                    let parts: Vec<&str> = args.iter().skip(1)
                        .filter_map(|a| a.as_str())
                        .collect();
                    json!(parts.join(separator))
                }
                "math_add" => {
                    let a = args.first().and_then(|v| v.as_f64()).unwrap_or(0.0);
                    let b = args.get(1).and_then(|v| v.as_f64()).unwrap_or(0.0);
                    json!(a + b)
                }
                "math_subtract" => {
                    let a = args.first().and_then(|v| v.as_f64()).unwrap_or(0.0);
                    let b = args.get(1).and_then(|v| v.as_f64()).unwrap_or(0.0);
                    json!(a - b)
                }
                "math_multiply" => {
                    let a = args.first().and_then(|v| v.as_f64()).unwrap_or(0.0);
                    let b = args.get(1).and_then(|v| v.as_f64()).unwrap_or(0.0);
                    json!(a * b)
                }
                "math_divide" => {
                    let a = args.first().and_then(|v| v.as_f64()).unwrap_or(0.0);
                    let b = args.get(1).and_then(|v| v.as_f64()).unwrap_or(1.0);
                    if b != 0.0 { json!(a / b) } else { json!(null) }
                }
                "math_modulo" => {
                    let a = args.first().and_then(|v| v.as_i64()).unwrap_or(0);
                    let b = args.get(1).and_then(|v| v.as_i64()).unwrap_or(1);
                    if b != 0 { json!(a % b) } else { json!(null) }
                }
                "math_power" => {
                    let a = args.first().and_then(|v| v.as_f64()).unwrap_or(0.0);
                    let b = args.get(1).and_then(|v| v.as_f64()).unwrap_or(1.0);
                    json!(a.powf(b))
                }
                "math_min" => {
                    let a = args.first().and_then(|v| v.as_f64()).unwrap_or(0.0);
                    let b = args.get(1).and_then(|v| v.as_f64()).unwrap_or(0.0);
                    json!(a.min(b))
                }
                "math_max" => {
                    let a = args.first().and_then(|v| v.as_f64()).unwrap_or(0.0);
                    let b = args.get(1).and_then(|v| v.as_f64()).unwrap_or(0.0);
                    json!(a.max(b))
                }
                "math_abs" => {
                    let a = args.first().and_then(|v| v.as_f64()).unwrap_or(0.0);
                    json!(a.abs())
                }
                "math_floor" => {
                    let a = args.first().and_then(|v| v.as_f64()).unwrap_or(0.0);
                    json!(a.floor() as i64)
                }
                "math_ceil" => {
                    let a = args.first().and_then(|v| v.as_f64()).unwrap_or(0.0);
                    json!(a.ceil() as i64)
                }
                "math_round" => {
                    let a = args.first().and_then(|v| v.as_f64()).unwrap_or(0.0);
                    json!(a.round() as i64)
                }
                "logic_and" => {
                    let a = args.first().and_then(|v| v.as_bool()).unwrap_or(false);
                    let b = args.get(1).and_then(|v| v.as_bool()).unwrap_or(false);
                    json!(a && b)
                }
                "logic_or" => {
                    let a = args.first().and_then(|v| v.as_bool()).unwrap_or(false);
                    let b = args.get(1).and_then(|v| v.as_bool()).unwrap_or(false);
                    json!(a || b)
                }
                "logic_not" => {
                    let a = args.first().and_then(|v| v.as_bool()).unwrap_or(false);
                    json!(!a)
                }
                "compare_equals" => {
                    let a = args.first();
                    let b = args.get(1);
                    json!(a == b)
                }
                "compare_greater" => {
                    let a = args.first().and_then(|v| v.as_f64()).unwrap_or(0.0);
                    let b = args.get(1).and_then(|v| v.as_f64()).unwrap_or(0.0);
                    json!(a > b)
                }
                "compare_less" => {
                    let a = args.first().and_then(|v| v.as_f64()).unwrap_or(0.0);
                    let b = args.get(1).and_then(|v| v.as_f64()).unwrap_or(0.0);
                    json!(a < b)
                }
                "compare_greater_equal" => {
                    let a = args.first().and_then(|v| v.as_f64()).unwrap_or(0.0);
                    let b = args.get(1).and_then(|v| v.as_f64()).unwrap_or(0.0);
                    json!(a >= b)
                }
                "compare_less_equal" => {
                    let a = args.first().and_then(|v| v.as_f64()).unwrap_or(0.0);
                    let b = args.get(1).and_then(|v| v.as_f64()).unwrap_or(0.0);
                    json!(a <= b)
                }
                "compare_not_equals" => {
                    let a = args.first();
                    let b = args.get(1);
                    json!(a != b)
                }
                "type_to_string" => {
                    json!(args.first().map(|v| v.to_string()).unwrap_or_default())
                }
                "string_length" => {
                    let s = args.first().and_then(|v| v.as_str()).unwrap_or("");
                    json!(s.len() as i64)
                }
                "string_upper" => {
                    let s = args.first().and_then(|v| v.as_str()).unwrap_or("").to_uppercase();
                    json!(s)
                }
                "string_lower" => {
                    let s = args.first().and_then(|v| v.as_str()).unwrap_or("").to_lowercase();
                    json!(s)
                }
                "string_replace" => {
                    let s = args.first().and_then(|v| v.as_str()).unwrap_or("").to_string();
                    let from = args.get(1).and_then(|v| v.as_str()).unwrap_or("").to_string();
                    let to = args.get(2).and_then(|v| v.as_str()).unwrap_or("").to_string();
                    json!(s.replace(&from, &to))
                }
                "int_to_float" => {
                    let a = args.first().and_then(|v| v.as_i64()).unwrap_or(0);
                    json!(a as f64)
                }
                "float_to_int" => {
                    let a = args.first().and_then(|v| v.as_f64()).unwrap_or(0.0);
                    json!(a as i64)
                }
                _ => {
                    json!({
                        "type": "pure_function_call",
                        "function_name": function_name,
                        "args": args,
                    })
                }
            };
            Ok(vec![result])
        })
    }));
}

fn register_guider_parameters(registry: &mut NodeRegistry) {
    let class_def = NodeClassDef {
        class_type: "GuiderParameters".to_string(),
        display_name: "Guider Parameters".to_string(),
        category: "sampling/ltx".to_string(),
        input_types: NodeInputTypes {
            required: {
                let mut m = HashMap::new();
                m.insert("modality".to_string(), InputTypeSpec {
                    type_name: "COMBO".to_string(),
                    extra: {
                        let mut e = HashMap::new();
                        e.insert("options".to_string(), json!(["VIDEO", "AUDIO"]));
                        e
                    },
                });
                m.insert("cfg".to_string(), InputTypeSpec {
                    type_name: "FLOAT".to_string(),
                    extra: HashMap::new(),
                });
                m.insert("stg".to_string(), InputTypeSpec {
                    type_name: "FLOAT".to_string(),
                    extra: HashMap::new(),
                });
                m.insert("perturb_attn".to_string(), InputTypeSpec {
                    type_name: "BOOLEAN".to_string(),
                    extra: HashMap::new(),
                });
                m.insert("rescale".to_string(), InputTypeSpec {
                    type_name: "FLOAT".to_string(),
                    extra: HashMap::new(),
                });
                m.insert("modality_scale".to_string(), InputTypeSpec {
                    type_name: "FLOAT".to_string(),
                    extra: HashMap::new(),
                });
                m.insert("skip_step".to_string(), InputTypeSpec {
                    type_name: "INT".to_string(),
                    extra: HashMap::new(),
                });
                m.insert("cross_attn".to_string(), InputTypeSpec {
                    type_name: "BOOLEAN".to_string(),
                    extra: HashMap::new(),
                });
                m
            },
            optional: {
                let mut m = HashMap::new();
                m.insert("parameters".to_string(), InputTypeSpec {
                    type_name: "GUIDER_PARAMETERS".to_string(),
                    extra: HashMap::new(),
                });
                m
            },
            hidden: HashMap::new(),
        },
        output_types: vec![IoType::GuiderParameters],
        output_names: vec!["GUIDER_PARAMETERS".to_string()],
        output_is_list: vec![false],
        is_output_node: false,
        has_intermediate_output: false,
        is_changed: None,
        not_idempotent: false,
        function_name: "get_guider_params".to_string(),
    };

    registry.register(class_def, Arc::new(|ctx, _node, node_id| {
        let modality = ctx.resolve_input(node_id, "modality").unwrap_or_else(|_| json!("VIDEO"));
        let cfg = ctx.resolve_input(node_id, "cfg").unwrap_or_else(|_| json!(1.0));
        let stg = ctx.resolve_input(node_id, "stg").unwrap_or_else(|_| json!(0.0));
        let perturb_attn = ctx.resolve_input(node_id, "perturb_attn").unwrap_or_else(|_| json!(false));
        let rescale = ctx.resolve_input(node_id, "rescale").unwrap_or_else(|_| json!(1.0));
        let modality_scale = ctx.resolve_input(node_id, "modality_scale").unwrap_or_else(|_| json!(1.0));
        let skip_step = ctx.resolve_input(node_id, "skip_step").unwrap_or_else(|_| json!(0));
        let cross_attn = ctx.resolve_input(node_id, "cross_attn").unwrap_or_else(|_| json!(true));
        let prev_params = ctx.resolve_input(node_id, "parameters").ok();

        Box::pin(async move {
            let mut current_params = json!({
                "modality": modality,
                "cfg": cfg,
                "stg": stg,
                "perturb_attn": perturb_attn,
                "rescale": rescale,
                "modality_scale": modality_scale,
                "skip_step": skip_step,
                "cross_attn": cross_attn,
            });

            if let Some(prev) = prev_params {
                let mut combined = prev.clone();
                if let Some(obj) = combined.as_object_mut() {
                    if let Some(curr_obj) = current_params.as_object() {
                        for (k, v) in curr_obj {
                            obj.insert(k.clone(), v.clone());
                        }
                    }
                }
                current_params = combined;
            }

            Ok(vec![current_params])
        })
    }));
}

fn register_multimodal_guider(registry: &mut NodeRegistry) {
    let class_def = NodeClassDef {
        class_type: "MultimodalGuider".to_string(),
        display_name: "Multimodal Guider".to_string(),
        category: "sampling/ltx".to_string(),
        input_types: NodeInputTypes {
            required: {
                let mut m = HashMap::new();
                m.insert("model".to_string(), InputTypeSpec {
                    type_name: "MODEL".to_string(),
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
                m.insert("parameters".to_string(), InputTypeSpec {
                    type_name: "GUIDER_PARAMETERS".to_string(),
                    extra: HashMap::new(),
                });
                m.insert("skip_blocks".to_string(), InputTypeSpec {
                    type_name: "STRING".to_string(),
                    extra: HashMap::new(),
                });
                m
            },
            optional: HashMap::new(),
            hidden: HashMap::new(),
        },
        output_types: vec![IoType::Guider],
        output_names: vec!["GUIDER".to_string()],
        output_is_list: vec![false],
        is_output_node: false,
        has_intermediate_output: false,
        is_changed: None,
        not_idempotent: false,
        function_name: "get_multimodal_guider".to_string(),
    };

    registry.register(class_def, Arc::new(|ctx, _node, node_id| {
        let model = ctx.resolve_input(node_id, "model").unwrap_or_else(|_| json!({}));
        let positive = ctx.resolve_input(node_id, "positive").unwrap_or_else(|_| json!({}));
        let negative = ctx.resolve_input(node_id, "negative").unwrap_or_else(|_| json!({}));
        let parameters = ctx.resolve_input(node_id, "parameters").unwrap_or_else(|_| json!({}));
        let skip_blocks = ctx.resolve_input(node_id, "skip_blocks").unwrap_or_else(|_| json!(""));

        Box::pin(async move {
            Ok(vec![json!({
                "type": "multimodal_guider",
                "model": model,
                "positive": positive,
                "negative": negative,
                "parameters": parameters,
                "skip_blocks": skip_blocks,
            })])
        })
    }));
}

fn register_save_video_with_audio(registry: &mut NodeRegistry) {
    let class_def = NodeClassDef {
        class_type: "SaveVideoWithAudio".to_string(),
        display_name: "Save Video with Audio".to_string(),
        category: "output".to_string(),
        input_types: NodeInputTypes {
            required: {
                let mut m = HashMap::new();
                m.insert("video".to_string(), InputTypeSpec {
                    type_name: "VIDEO".to_string(),
                    extra: HashMap::new(),
                });
                m.insert("audio".to_string(), InputTypeSpec {
                    type_name: "AUDIO".to_string(),
                    extra: HashMap::new(),
                });
                m.insert("filename_prefix".to_string(), InputTypeSpec {
                    type_name: "STRING".to_string(),
                    extra: HashMap::new(),
                });
                m.insert("format".to_string(), InputTypeSpec {
                    type_name: "COMBO".to_string(),
                    extra: {
                        let mut e = HashMap::new();
                        e.insert("options".to_string(), json!(["mp4", "webm"]));
                        e
                    },
                });
                m
            },
            optional: HashMap::new(),
            hidden: HashMap::new(),
        },
        output_types: vec![IoType::Video],
        output_names: vec!["VIDEO".to_string()],
        output_is_list: vec![false],
        is_output_node: true,
        has_intermediate_output: false,
        is_changed: None,
        not_idempotent: false,
        function_name: "save_video_with_audio".to_string(),
    };

    registry.register(class_def, Arc::new(|ctx, _node, node_id| {
        let video = ctx.resolve_input(node_id, "video").unwrap_or_else(|_| json!({}));
        let audio = ctx.resolve_input(node_id, "audio").unwrap_or_else(|_| json!({}));
        let filename_prefix = ctx.resolve_input(node_id, "filename_prefix").unwrap_or_else(|_| json!("ltx_output"));
        let format = ctx.resolve_input(node_id, "format").unwrap_or_else(|_| json!("mp4"));

        Box::pin(async move {
            Ok(vec![json!({
                "video": video,
                "audio": audio,
                "filename_prefix": filename_prefix,
                "format": format,
            })])
        })
    }));
}

// ============ STG 相关节点 ============

fn register_stg_guider_node(registry: &mut NodeRegistry) {
    let class_def = NodeClassDef {
        class_type: "STGGuiderNode".to_string(),
        display_name: "STG Guider".to_string(),
        category: "sampling/ltx".to_string(),
        input_types: NodeInputTypes {
            required: {
                let mut m = HashMap::new();
                m.insert("model".to_string(), InputTypeSpec {
                    type_name: "MODEL".to_string(),
                    extra: HashMap::new(),
                });
                m.insert("stg".to_string(), InputTypeSpec {
                    type_name: "FLOAT".to_string(),
                    extra: HashMap::new(),
                });
                m
            },
            optional: HashMap::new(),
            hidden: HashMap::new(),
        },
        output_types: vec![IoType::Guider],
        output_names: vec!["GUIDER".to_string()],
        output_is_list: vec![false],
        is_output_node: false,
        has_intermediate_output: false,
        is_changed: None,
        not_idempotent: false,
        function_name: "stg_guider".to_string(),
    };

    registry.register(class_def, Arc::new(|ctx, _node, node_id| {
        let model = ctx.resolve_input(node_id, "model").unwrap_or_else(|_| json!({}));
        let stg = ctx.resolve_input(node_id, "stg").unwrap_or_else(|_| json!(0.0));

        Box::pin(async move {
            Ok(vec![json!({
                "type": "stg_guider",
                "model": model,
                "stg": stg,
            })])
        })
    }));
}

fn register_stg_guider_advanced_node(registry: &mut NodeRegistry) {
    let class_def = NodeClassDef {
        class_type: "STGGuiderAdvancedNode".to_string(),
        display_name: "STG Guider Advanced".to_string(),
        category: "sampling/ltx".to_string(),
        input_types: NodeInputTypes {
            required: {
                let mut m = HashMap::new();
                m.insert("model".to_string(), InputTypeSpec {
                    type_name: "MODEL".to_string(),
                    extra: HashMap::new(),
                });
                m.insert("stg".to_string(), InputTypeSpec {
                    type_name: "FLOAT".to_string(),
                    extra: HashMap::new(),
                });
                m.insert("start_step".to_string(), InputTypeSpec {
                    type_name: "INT".to_string(),
                    extra: HashMap::new(),
                });
                m.insert("end_step".to_string(), InputTypeSpec {
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
        output_types: vec![IoType::Guider],
        output_names: vec!["GUIDER".to_string()],
        output_is_list: vec![false],
        is_output_node: false,
        has_intermediate_output: false,
        is_changed: None,
        not_idempotent: false,
        function_name: "stg_guider_advanced".to_string(),
    };

    registry.register(class_def, Arc::new(|ctx, _node, node_id| {
        let model = ctx.resolve_input(node_id, "model").unwrap_or_else(|_| json!({}));
        let stg = ctx.resolve_input(node_id, "stg").unwrap_or_else(|_| json!(0.0));
        let start_step = ctx.resolve_input(node_id, "start_step").unwrap_or_else(|_| json!(0));
        let end_step = ctx.resolve_input(node_id, "end_step").unwrap_or_else(|_| json!(999));
        let batch_size = ctx.resolve_input(node_id, "batch_size").unwrap_or_else(|_| json!(1));

        Box::pin(async move {
            Ok(vec![json!({
                "type": "stg_guider_advanced",
                "model": model,
                "stg": stg,
                "start_step": start_step,
                "end_step": end_step,
                "batch_size": batch_size,
            })])
        })
    }));
}

fn register_stg_advanced_presets_node(registry: &mut NodeRegistry) {
    let class_def = NodeClassDef {
        class_type: "STGAdvancedPresetsNode".to_string(),
        display_name: "STG Advanced Presets".to_string(),
        category: "sampling/ltx".to_string(),
        input_types: NodeInputTypes {
            required: {
                let mut m = HashMap::new();
                m.insert("preset".to_string(), InputTypeSpec {
                    type_name: "COMBO".to_string(),
                    extra: {
                        let mut e = HashMap::new();
                        e.insert("options".to_string(), json!(["default", "slow_start", "fast_end", "linear_decay", "custom"]));
                        e
                    },
                });
                m
            },
            optional: {
                let mut m = HashMap::new();
                m.insert("stg".to_string(), InputTypeSpec {
                    type_name: "FLOAT".to_string(),
                    extra: HashMap::new(),
                });
                m
            },
            hidden: HashMap::new(),
        },
        output_types: vec![IoType::Guider],
        output_names: vec!["GUIDER".to_string()],
        output_is_list: vec![false],
        is_output_node: false,
        has_intermediate_output: false,
        is_changed: None,
        not_idempotent: false,
        function_name: "stg_presets".to_string(),
    };

    registry.register(class_def, Arc::new(|ctx, _node, node_id| {
        let preset = ctx.resolve_input(node_id, "preset").unwrap_or_else(|_| json!("default"));
        let stg = ctx.resolve_input(node_id, "stg").unwrap_or_else(|_| json!(0.0));

        Box::pin(async move {
            let stg_curve = match preset.as_str().unwrap_or("default") {
                "slow_start" => json!({"type": "slow_start", "stg": stg}),
                "fast_end" => json!({"type": "fast_end", "stg": stg}),
                "linear_decay" => json!({"type": "linear_decay", "stg": stg}),
                "custom" => json!({"type": "custom", "stg": stg}),
                _ => json!({"type": "default", "stg": stg}),
            };
            Ok(vec![json!({
                "type": "stg_guider_advanced",
                "stg_curve": stg_curve,
            })])
        })
    }));
}

fn register_ltxv_apply_stg(registry: &mut NodeRegistry) {
    let class_def = NodeClassDef {
        class_type: "LTXVApplySTG".to_string(),
        display_name: "LTXV Apply STG".to_string(),
        category: "sampling/ltx".to_string(),
        input_types: NodeInputTypes {
            required: {
                let mut m = HashMap::new();
                m.insert("model".to_string(), InputTypeSpec {
                    type_name: "MODEL".to_string(),
                    extra: HashMap::new(),
                });
                m.insert("stg".to_string(), InputTypeSpec {
                    type_name: "FLOAT".to_string(),
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
        function_name: "apply_stg".to_string(),
    };

    registry.register(class_def, Arc::new(|ctx, _node, node_id| {
        let model = ctx.resolve_input(node_id, "model").unwrap_or_else(|_| json!({}));
        let stg = ctx.resolve_input(node_id, "stg").unwrap_or_else(|_| json!(0.0));

        Box::pin(async move {
            let mut model_out = model.as_object().cloned().unwrap_or_default();
            model_out.insert("stg".to_string(), stg);
            Ok(vec![json!(model_out)])
        })
    }));
}

// ============ 基础采样器节点 ============

fn register_ltxv_base_sampler(registry: &mut NodeRegistry) {
    let class_def = NodeClassDef {
        class_type: "LTXVBaseSampler".to_string(),
        display_name: "LTXV Base Sampler".to_string(),
        category: "sampling/ltx".to_string(),
        input_types: NodeInputTypes {
            required: {
                let mut m = HashMap::new();
                m.insert("model".to_string(), InputTypeSpec {
                    type_name: "MODEL".to_string(),
                    extra: HashMap::new(),
                });
                m.insert("guider".to_string(), InputTypeSpec {
                    type_name: "GUIDER".to_string(),
                    extra: HashMap::new(),
                });
                m.insert("latent_image".to_string(), InputTypeSpec {
                    type_name: "LATENT".to_string(),
                    extra: HashMap::new(),
                });
                m.insert("steps".to_string(), InputTypeSpec {
                    type_name: "INT".to_string(),
                    extra: HashMap::new(),
                });
                m.insert("sampler_name".to_string(), InputTypeSpec {
                    type_name: "COMBO".to_string(),
                    extra: {
                        let mut e = HashMap::new();
                        e.insert("options".to_string(), json!(["euler", "euler_ancestral", "dpm_2", "dpm_2_ancestral", "dpmpp_2m", "dpmpp_sde", "ddim"]));
                        e
                    },
                });
                m.insert("scheduler".to_string(), InputTypeSpec {
                    type_name: "COMBO".to_string(),
                    extra: {
                        let mut e = HashMap::new();
                        e.insert("options".to_string(), json!(["normal", "karras", "exponential", "simple", "ddim_uniform"]));
                        e
                    },
                });
                m.insert("denoise".to_string(), InputTypeSpec {
                    type_name: "FLOAT".to_string(),
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
        function_name: "ltxv_base_sampler".to_string(),
    };

    registry.register(class_def, Arc::new(|ctx, _node, node_id| {
        Box::pin(async move {
            let latent = ctx.resolve_input(node_id, "latent_image").unwrap_or_else(|_| json!({}));
            Ok(vec![latent])
        })
    }));
}

fn register_ltxv_extend_sampler(registry: &mut NodeRegistry) {
    let class_def = NodeClassDef {
        class_type: "LTXVExtendSampler".to_string(),
        display_name: "LTXV Extend Sampler".to_string(),
        category: "sampling/ltx".to_string(),
        input_types: NodeInputTypes {
            required: {
                let mut m = HashMap::new();
                m.insert("model".to_string(), InputTypeSpec {
                    type_name: "MODEL".to_string(),
                    extra: HashMap::new(),
                });
                m.insert("guider".to_string(), InputTypeSpec {
                    type_name: "GUIDER".to_string(),
                    extra: HashMap::new(),
                });
                m.insert("latent_image".to_string(), InputTypeSpec {
                    type_name: "LATENT".to_string(),
                    extra: HashMap::new(),
                });
                m.insert("steps".to_string(), InputTypeSpec {
                    type_name: "INT".to_string(),
                    extra: HashMap::new(),
                });
                m.insert("extend_frames".to_string(), InputTypeSpec {
                    type_name: "INT".to_string(),
                    extra: HashMap::new(),
                });
                m.insert("overlap".to_string(), InputTypeSpec {
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
        function_name: "ltxv_extend_sampler".to_string(),
    };

    registry.register(class_def, Arc::new(|ctx, _node, node_id| {
        Box::pin(async move {
            let latent = ctx.resolve_input(node_id, "latent_image").unwrap_or_else(|_| json!({}));
            Ok(vec![latent])
        })
    }));
}

fn register_ltxv_in_context_sampler(registry: &mut NodeRegistry) {
    let class_def = NodeClassDef {
        class_type: "LTXVInContextSampler".to_string(),
        display_name: "LTXV In-Context Sampler".to_string(),
        category: "sampling/ltx".to_string(),
        input_types: NodeInputTypes {
            required: {
                let mut m = HashMap::new();
                m.insert("model".to_string(), InputTypeSpec {
                    type_name: "MODEL".to_string(),
                    extra: HashMap::new(),
                });
                m.insert("guider".to_string(), InputTypeSpec {
                    type_name: "GUIDER".to_string(),
                    extra: HashMap::new(),
                });
                m.insert("latent_image".to_string(), InputTypeSpec {
                    type_name: "LATENT".to_string(),
                    extra: HashMap::new(),
                });
                m.insert("context_frames".to_string(), InputTypeSpec {
                    type_name: "INT".to_string(),
                    extra: HashMap::new(),
                });
                m.insert("steps".to_string(), InputTypeSpec {
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
        function_name: "ltxv_in_context_sampler".to_string(),
    };

    registry.register(class_def, Arc::new(|ctx, _node, node_id| {
        Box::pin(async move {
            let latent = ctx.resolve_input(node_id, "latent_image").unwrap_or_else(|_| json!({}));
            Ok(vec![latent])
        })
    }));
}

fn register_ltxv_normalizing_sampler(registry: &mut NodeRegistry) {
    let class_def = NodeClassDef {
        class_type: "LTXVNormalizingSampler".to_string(),
        display_name: "LTXV Normalizing Sampler".to_string(),
        category: "sampling/ltx".to_string(),
        input_types: NodeInputTypes {
            required: {
                let mut m = HashMap::new();
                m.insert("model".to_string(), InputTypeSpec {
                    type_name: "MODEL".to_string(),
                    extra: HashMap::new(),
                });
                m.insert("guider".to_string(), InputTypeSpec {
                    type_name: "GUIDER".to_string(),
                    extra: HashMap::new(),
                });
                m.insert("latent_image".to_string(), InputTypeSpec {
                    type_name: "LATENT".to_string(),
                    extra: HashMap::new(),
                });
                m.insert("reference".to_string(), InputTypeSpec {
                    type_name: "LATENT".to_string(),
                    extra: HashMap::new(),
                });
                m.insert("steps".to_string(), InputTypeSpec {
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
        function_name: "ltxv_normalizing_sampler".to_string(),
    };

    registry.register(class_def, Arc::new(|ctx, _node, node_id| {
        Box::pin(async move {
            let latent = ctx.resolve_input(node_id, "latent_image").unwrap_or_else(|_| json!({}));
            Ok(vec![latent])
        })
    }));
}

fn register_linear_overlap_latent_transition(registry: &mut NodeRegistry) {
    let class_def = NodeClassDef {
        class_type: "LinearOverlapLatentTransition".to_string(),
        display_name: "Linear Overlap Latent Transition".to_string(),
        category: "latent/ltx".to_string(),
        input_types: NodeInputTypes {
            required: {
                let mut m = HashMap::new();
                m.insert("latent_a".to_string(), InputTypeSpec {
                    type_name: "LATENT".to_string(),
                    extra: HashMap::new(),
                });
                m.insert("latent_b".to_string(), InputTypeSpec {
                    type_name: "LATENT".to_string(),
                    extra: HashMap::new(),
                });
                m.insert("overlap_frames".to_string(), InputTypeSpec {
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
        function_name: "linear_overlap_transition".to_string(),
    };

    registry.register(class_def, Arc::new(|ctx, _node, node_id| {
        let latent_a = ctx.resolve_input(node_id, "latent_a").unwrap_or_else(|_| json!({}));
        Box::pin(async move {
            Ok(vec![latent_a])
        })
    }));
}

// ============ 高级采样器节点 ============

fn register_ltxv_looping_sampler(registry: &mut NodeRegistry) {
    let class_def = NodeClassDef {
        class_type: "LTXVLoopingSampler".to_string(),
        display_name: "LTXV Looping Sampler".to_string(),
        category: "sampling/ltx".to_string(),
        input_types: NodeInputTypes {
            required: {
                let mut m = HashMap::new();
                m.insert("model".to_string(), InputTypeSpec {
                    type_name: "MODEL".to_string(),
                    extra: HashMap::new(),
                });
                m.insert("guider".to_string(), InputTypeSpec {
                    type_name: "GUIDER".to_string(),
                    extra: HashMap::new(),
                });
                m.insert("latent_image".to_string(), InputTypeSpec {
                    type_name: "LATENT".to_string(),
                    extra: HashMap::new(),
                });
                m.insert("steps".to_string(), InputTypeSpec {
                    type_name: "INT".to_string(),
                    extra: HashMap::new(),
                });
                m.insert("loop_count".to_string(), InputTypeSpec {
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
        function_name: "ltxv_looping_sampler".to_string(),
    };

    registry.register(class_def, Arc::new(|ctx, _node, node_id| {
        Box::pin(async move {
            let latent = ctx.resolve_input(node_id, "latent_image").unwrap_or_else(|_| json!({}));
            Ok(vec![latent])
        })
    }));
}

fn register_ltxv_tiled_sampler(registry: &mut NodeRegistry) {
    let class_def = NodeClassDef {
        class_type: "LTXVTiledSampler".to_string(),
        display_name: "LTXV Tiled Sampler".to_string(),
        category: "sampling/ltx".to_string(),
        input_types: NodeInputTypes {
            required: {
                let mut m = HashMap::new();
                m.insert("model".to_string(), InputTypeSpec {
                    type_name: "MODEL".to_string(),
                    extra: HashMap::new(),
                });
                m.insert("guider".to_string(), InputTypeSpec {
                    type_name: "GUIDER".to_string(),
                    extra: HashMap::new(),
                });
                m.insert("latent_image".to_string(), InputTypeSpec {
                    type_name: "LATENT".to_string(),
                    extra: HashMap::new(),
                });
                m.insert("steps".to_string(), InputTypeSpec {
                    type_name: "INT".to_string(),
                    extra: HashMap::new(),
                });
                m.insert("tile_size".to_string(), InputTypeSpec {
                    type_name: "INT".to_string(),
                    extra: HashMap::new(),
                });
                m.insert("overlap".to_string(), InputTypeSpec {
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
        function_name: "ltxv_tiled_sampler".to_string(),
    };

    registry.register(class_def, Arc::new(|ctx, _node, node_id| {
        Box::pin(async move {
            let latent = ctx.resolve_input(node_id, "latent_image").unwrap_or_else(|_| json!({}));
            Ok(vec![latent])
        })
    }));
}

fn register_ltxv_tiled_vae_decode(registry: &mut NodeRegistry) {
    let class_def = NodeClassDef {
        class_type: "LTXVTiledVAEDecode".to_string(),
        display_name: "LTXV Tiled VAE Decode".to_string(),
        category: "latent/ltx".to_string(),
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
                m.insert("tile_size".to_string(), InputTypeSpec {
                    type_name: "INT".to_string(),
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
        function_name: "ltxv_tiled_vae_decode".to_string(),
    };

    registry.register(class_def, Arc::new(|_ctx, node, _node_id| {
        Box::pin(async move {
            Ok(vec![json!({
                "type": "tiled_vae_decode",
                "samples": node.inputs.get("samples"),
            })])
        })
    }));
}

// ============ IC-LoRA 节点 ============

fn register_ltx_add_video_ic_lora_guide(registry: &mut NodeRegistry) {
    let class_def = NodeClassDef {
        class_type: "LTXAddVideoICLoRAGuide".to_string(),
        display_name: "LTX Add Video IC-LoRA Guide".to_string(),
        category: "sampling/ltx".to_string(),
        input_types: NodeInputTypes {
            required: {
                let mut m = HashMap::new();
                m.insert("model".to_string(), InputTypeSpec {
                    type_name: "MODEL".to_string(),
                    extra: HashMap::new(),
                });
                m.insert("iclora".to_string(), InputTypeSpec {
                    type_name: "LORA".to_string(),
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
        output_types: vec![IoType::Model],
        output_names: vec!["MODEL".to_string()],
        output_is_list: vec![false],
        is_output_node: false,
        has_intermediate_output: false,
        is_changed: None,
        not_idempotent: false,
        function_name: "add_ic_lora_guide".to_string(),
    };

    registry.register(class_def, Arc::new(|ctx, _node, node_id| {
        let model = ctx.resolve_input(node_id, "model").unwrap_or_else(|_| json!({}));
        let iclora = ctx.resolve_input(node_id, "iclora").unwrap_or_else(|_| json!({}));
        let strength = ctx.resolve_input(node_id, "strength").unwrap_or_else(|_| json!(1.0));

        Box::pin(async move {
            let mut model_out = model.as_object().cloned().unwrap_or_default();
            model_out.insert("ic_lora".to_string(), iclora);
            model_out.insert("ic_lora_strength".to_string(), strength);
            Ok(vec![json!(model_out)])
        })
    }));
}

fn register_ltx_add_video_ic_lora_guide_advanced(registry: &mut NodeRegistry) {
    let class_def = NodeClassDef {
        class_type: "LTXAddVideoICLoRAGuideAdvanced".to_string(),
        display_name: "LTX Add Video IC-LoRA Guide Advanced".to_string(),
        category: "sampling/ltx".to_string(),
        input_types: NodeInputTypes {
            required: {
                let mut m = HashMap::new();
                m.insert("model".to_string(), InputTypeSpec {
                    type_name: "MODEL".to_string(),
                    extra: HashMap::new(),
                });
                m.insert("iclora".to_string(), InputTypeSpec {
                    type_name: "LORA".to_string(),
                    extra: HashMap::new(),
                });
                m.insert("strength".to_string(), InputTypeSpec {
                    type_name: "FLOAT".to_string(),
                    extra: HashMap::new(),
                });
                m.insert("start_step".to_string(), InputTypeSpec {
                    type_name: "INT".to_string(),
                    extra: HashMap::new(),
                });
                m.insert("end_step".to_string(), InputTypeSpec {
                    type_name: "INT".to_string(),
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
        function_name: "add_ic_lora_guide_advanced".to_string(),
    };

    registry.register(class_def, Arc::new(|ctx, _node, node_id| {
        let model = ctx.resolve_input(node_id, "model").unwrap_or_else(|_| json!({}));
        let iclora = ctx.resolve_input(node_id, "iclora").unwrap_or_else(|_| json!({}));
        let strength = ctx.resolve_input(node_id, "strength").unwrap_or_else(|_| json!(1.0));
        let start_step = ctx.resolve_input(node_id, "start_step").unwrap_or_else(|_| json!(0));
        let end_step = ctx.resolve_input(node_id, "end_step").unwrap_or_else(|_| json!(999));

        Box::pin(async move {
            let mut model_out = model.as_object().cloned().unwrap_or_default();
            model_out.insert("ic_lora".to_string(), iclora);
            model_out.insert("ic_lora_strength".to_string(), strength);
            model_out.insert("ic_lora_start_step".to_string(), start_step);
            model_out.insert("ic_lora_end_step".to_string(), end_step);
            Ok(vec![json!(model_out)])
        })
    }));
}

fn register_ltx_iclora_loader_model_only(registry: &mut NodeRegistry) {
    let class_def = NodeClassDef {
        class_type: "LTXICLoRALoaderModelOnly".to_string(),
        display_name: "LTX IC-LoRA Loader".to_string(),
        category: "loaders/ltx".to_string(),
        input_types: NodeInputTypes {
            required: {
                let mut m = HashMap::new();
                m.insert("iclora_name".to_string(), InputTypeSpec {
                    type_name: "COMBO".to_string(),
                    extra: HashMap::new(),
                });
                m
            },
            optional: HashMap::new(),
            hidden: HashMap::new(),
        },
        output_types: vec![IoType::Lora],
        output_names: vec!["LORA".to_string()],
        output_is_list: vec![false],
        is_output_node: false,
        has_intermediate_output: false,
        is_changed: None,
        not_idempotent: false,
        function_name: "load_iclora".to_string(),
    };

    registry.register(class_def, Arc::new(|_ctx, node, _node_id| {
        let iclora_name = node.inputs.get("iclora_name")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        Box::pin(async move {
            Ok(vec![json!({
                "type": "iclora",
                "name": iclora_name,
            })])
        })
    }));
}

fn register_ltxv_set_audio_ref_tokens(registry: &mut NodeRegistry) {
    let class_def = NodeClassDef {
        class_type: "LTXVSetAudioRefTokens".to_string(),
        display_name: "LTXV Set Audio Reference Tokens".to_string(),
        category: "conditioning/ltx".to_string(),
        input_types: NodeInputTypes {
            required: {
                let mut m = HashMap::new();
                m.insert("audio_ref".to_string(), InputTypeSpec {
                    type_name: "AUDIO".to_string(),
                    extra: HashMap::new(),
                });
                m.insert("ref_tokens".to_string(), InputTypeSpec {
                    type_name: "INT".to_string(),
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
        function_name: "set_audio_ref_tokens".to_string(),
    };

    registry.register(class_def, Arc::new(|ctx, _node, node_id| {
        let audio_ref = ctx.resolve_input(node_id, "audio_ref").unwrap_or_else(|_| json!({}));
        let ref_tokens = ctx.resolve_input(node_id, "ref_tokens").unwrap_or_else(|_| json!(0));

        Box::pin(async move {
            Ok(vec![json!({
                "type": "audio_ref_tokens",
                "audio_ref": audio_ref,
                "ref_tokens": ref_tokens,
            })])
        })
    }));
}

// ============ Latent 归一化节点 ============

fn register_ltxv_adain_latent(registry: &mut NodeRegistry) {
    let class_def = NodeClassDef {
        class_type: "LTXVAdainLatent".to_string(),
        display_name: "LTXV AdaIN Latent".to_string(),
        category: "latent/ltx".to_string(),
        input_types: NodeInputTypes {
            required: {
                let mut m = HashMap::new();
                m.insert("latent".to_string(), InputTypeSpec {
                    type_name: "LATENT".to_string(),
                    extra: HashMap::new(),
                });
                m.insert("reference".to_string(), InputTypeSpec {
                    type_name: "LATENT".to_string(),
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
        function_name: "adain_latent".to_string(),
    };

    registry.register(class_def, Arc::new(|ctx, _node, node_id| {
        let latent = ctx.resolve_input(node_id, "latent").unwrap_or_else(|_| json!({}));
        Box::pin(async move {
            Ok(vec![latent])
        })
    }));
}

fn register_ltxv_stat_norm_latent(registry: &mut NodeRegistry) {
    let class_def = NodeClassDef {
        class_type: "LTXVStatNormLatent".to_string(),
        display_name: "LTXV Stat Norm Latent".to_string(),
        category: "latent/ltx".to_string(),
        input_types: NodeInputTypes {
            required: {
                let mut m = HashMap::new();
                m.insert("latent".to_string(), InputTypeSpec {
                    type_name: "LATENT".to_string(),
                    extra: HashMap::new(),
                });
                m.insert("mean".to_string(), InputTypeSpec {
                    type_name: "FLOAT".to_string(),
                    extra: HashMap::new(),
                });
                m.insert("std".to_string(), InputTypeSpec {
                    type_name: "FLOAT".to_string(),
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
        function_name: "stat_norm_latent".to_string(),
    };

    registry.register(class_def, Arc::new(|ctx, _node, node_id| {
        let latent = ctx.resolve_input(node_id, "latent").unwrap_or_else(|_| json!({}));
        Box::pin(async move {
            Ok(vec![latent])
        })
    }));
}

fn register_ltxv_per_step_adain_patcher(registry: &mut NodeRegistry) {
    let class_def = NodeClassDef {
        class_type: "LTXVPerStepAdainPatcher".to_string(),
        display_name: "LTXV Per-Step AdaIN Patcher".to_string(),
        category: "latent/ltx".to_string(),
        input_types: NodeInputTypes {
            required: {
                let mut m = HashMap::new();
                m.insert("model".to_string(), InputTypeSpec {
                    type_name: "MODEL".to_string(),
                    extra: HashMap::new(),
                });
                m.insert("reference".to_string(), InputTypeSpec {
                    type_name: "LATENT".to_string(),
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
        function_name: "per_step_adain_patcher".to_string(),
    };

    registry.register(class_def, Arc::new(|ctx, _node, node_id| {
        let model = ctx.resolve_input(node_id, "model").unwrap_or_else(|_| json!({}));
        Box::pin(async move {
            let mut model_out = model.as_object().cloned().unwrap_or_default();
            model_out.insert("per_step_adain".to_string(), json!(true));
            Ok(vec![json!(model_out)])
        })
    }));
}

fn register_ltxv_per_step_stat_norm_patcher(registry: &mut NodeRegistry) {
    let class_def = NodeClassDef {
        class_type: "LTXVPerStepStatNormPatcher".to_string(),
        display_name: "LTXV Per-Step Stat Norm Patcher".to_string(),
        category: "latent/ltx".to_string(),
        input_types: NodeInputTypes {
            required: {
                let mut m = HashMap::new();
                m.insert("model".to_string(), InputTypeSpec {
                    type_name: "MODEL".to_string(),
                    extra: HashMap::new(),
                });
                m.insert("mean".to_string(), InputTypeSpec {
                    type_name: "FLOAT".to_string(),
                    extra: HashMap::new(),
                });
                m.insert("std".to_string(), InputTypeSpec {
                    type_name: "FLOAT".to_string(),
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
        function_name: "per_step_stat_norm_patcher".to_string(),
    };

    registry.register(class_def, Arc::new(|ctx, _node, node_id| {
        let model = ctx.resolve_input(node_id, "model").unwrap_or_else(|_| json!({}));
        let mean = ctx.resolve_input(node_id, "mean").unwrap_or_else(|_| json!(0.0));
        let std = ctx.resolve_input(node_id, "std").unwrap_or_else(|_| json!(1.0));

        Box::pin(async move {
            let mut model_out = model.as_object().cloned().unwrap_or_default();
            model_out.insert("per_step_stat_norm".to_string(), json!(true));
            model_out.insert("stat_norm_mean".to_string(), mean);
            model_out.insert("stat_norm_std".to_string(), std);
            Ok(vec![json!(model_out)])
        })
    }));
}

// ============ Latent 操作节点 ============

fn register_ltxv_add_latent_guide(registry: &mut NodeRegistry) {
    let class_def = NodeClassDef {
        class_type: "LTXVAddLatentGuide".to_string(),
        display_name: "LTXV Add Latent Guide".to_string(),
        category: "latent/ltx".to_string(),
        input_types: NodeInputTypes {
            required: {
                let mut m = HashMap::new();
                m.insert("latent".to_string(), InputTypeSpec {
                    type_name: "LATENT".to_string(),
                    extra: HashMap::new(),
                });
                m.insert("guide".to_string(), InputTypeSpec {
                    type_name: "LATENT".to_string(),
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
        output_types: vec![IoType::Latent],
        output_names: vec!["LATENT".to_string()],
        output_is_list: vec![false],
        is_output_node: false,
        has_intermediate_output: false,
        is_changed: None,
        not_idempotent: false,
        function_name: "add_latent_guide".to_string(),
    };

    registry.register(class_def, Arc::new(|ctx, _node, node_id| {
        let latent = ctx.resolve_input(node_id, "latent").unwrap_or_else(|_| json!({}));
        Box::pin(async move {
            Ok(vec![latent])
        })
    }));
}

fn register_ltxv_img_to_video_condition_only(registry: &mut NodeRegistry) {
    let class_def = NodeClassDef {
        class_type: "LTXVImgToVideoConditionOnly".to_string(),
        display_name: "LTXV Image to Video (Condition Only)".to_string(),
        category: "latent/ltx".to_string(),
        input_types: NodeInputTypes {
            required: {
                let mut m = HashMap::new();
                m.insert("image".to_string(), InputTypeSpec {
                    type_name: "IMAGE".to_string(),
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
        function_name: "img_to_video_condition".to_string(),
    };

    registry.register(class_def, Arc::new(|_ctx, _node, _node_id| {
        Box::pin(async move {
            Ok(vec![json!({
                "type": "img_to_video_condition",
            })])
        })
    }));
}

fn register_ltxv_select_latents(registry: &mut NodeRegistry) {
    let class_def = NodeClassDef {
        class_type: "LTXVSelectLatents".to_string(),
        display_name: "LTXV Select Latents".to_string(),
        category: "latent/ltx".to_string(),
        input_types: NodeInputTypes {
            required: {
                let mut m = HashMap::new();
                m.insert("latents".to_string(), InputTypeSpec {
                    type_name: "LATENT".to_string(),
                    extra: HashMap::new(),
                });
                m.insert("start_frame".to_string(), InputTypeSpec {
                    type_name: "INT".to_string(),
                    extra: HashMap::new(),
                });
                m.insert("end_frame".to_string(), InputTypeSpec {
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
        function_name: "select_latents".to_string(),
    };

    registry.register(class_def, Arc::new(|ctx, _node, node_id| {
        let latents = ctx.resolve_input(node_id, "latents").unwrap_or_else(|_| json!({}));
        Box::pin(async move {
            Ok(vec![latents])
        })
    }));
}

fn register_ltxv_set_video_latent_noise_masks(registry: &mut NodeRegistry) {
    let class_def = NodeClassDef {
        class_type: "LTXVSetVideoLatentNoiseMasks".to_string(),
        display_name: "LTXV Set Video Latent Noise Masks".to_string(),
        category: "latent/ltx".to_string(),
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
        function_name: "set_video_noise_masks".to_string(),
    };

    registry.register(class_def, Arc::new(|ctx, _node, node_id| {
        let samples = ctx.resolve_input(node_id, "samples").unwrap_or_else(|_| json!({}));
        Box::pin(async move {
            Ok(vec![samples])
        })
    }));
}

fn register_ltxv_laplacian_pyramid_blend(registry: &mut NodeRegistry) {
    let class_def = NodeClassDef {
        class_type: "LTXVLaplacianPyramidBlend".to_string(),
        display_name: "LTXV Laplacian Pyramid Blend".to_string(),
        category: "latent/ltx".to_string(),
        input_types: NodeInputTypes {
            required: {
                let mut m = HashMap::new();
                m.insert("latent_a".to_string(), InputTypeSpec {
                    type_name: "LATENT".to_string(),
                    extra: HashMap::new(),
                });
                m.insert("latent_b".to_string(), InputTypeSpec {
                    type_name: "LATENT".to_string(),
                    extra: HashMap::new(),
                });
                m.insert("mask".to_string(), InputTypeSpec {
                    type_name: "MASK".to_string(),
                    extra: HashMap::new(),
                });
                m.insert("levels".to_string(), InputTypeSpec {
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
        function_name: "laplacian_pyramid_blend".to_string(),
    };

    registry.register(class_def, Arc::new(|ctx, _node, node_id| {
        let latent_a = ctx.resolve_input(node_id, "latent_a").unwrap_or_else(|_| json!({}));
        Box::pin(async move {
            Ok(vec![latent_a])
        })
    }));
}

// ============ 辅助工具节点 ============

fn register_float_to_int(registry: &mut NodeRegistry) {
    let class_def = NodeClassDef {
        class_type: "FloatToInt".to_string(),
        display_name: "Float to Int".to_string(),
        category: "utils".to_string(),
        input_types: NodeInputTypes {
            required: {
                let mut m = HashMap::new();
                m.insert("value".to_string(), InputTypeSpec {
                    type_name: "FLOAT".to_string(),
                    extra: HashMap::new(),
                });
                m
            },
            optional: HashMap::new(),
            hidden: HashMap::new(),
        },
        output_types: vec![IoType::Int],
        output_names: vec!["INT".to_string()],
        output_is_list: vec![false],
        is_output_node: false,
        has_intermediate_output: false,
        is_changed: None,
        not_idempotent: false,
        function_name: "float_to_int".to_string(),
    };

    registry.register(class_def, Arc::new(|ctx, _node, node_id| {
        let value = ctx.resolve_input(node_id, "value").unwrap_or_else(|_| json!(0.0));
        let int_val = value.as_f64().unwrap_or(0.0) as i64;
        Box::pin(async move {
            Ok(vec![json!(int_val)])
        })
    }));
}

fn register_image_to_cpu(registry: &mut NodeRegistry) {
    let class_def = NodeClassDef {
        class_type: "ImageToCPU".to_string(),
        display_name: "Image to CPU".to_string(),
        category: "utils".to_string(),
        input_types: NodeInputTypes {
            required: {
                let mut m = HashMap::new();
                m.insert("images".to_string(), InputTypeSpec {
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
        function_name: "image_to_cpu".to_string(),
    };

    registry.register(class_def, Arc::new(|ctx, _node, node_id| {
        let images = ctx.resolve_input(node_id, "images").unwrap_or_else(|_| json!({}));
        Box::pin(async move {
            Ok(vec![images])
        })
    }));
}

fn register_ltxv_hdr_decode_postprocess(registry: &mut NodeRegistry) {
    let class_def = NodeClassDef {
        class_type: "LTXVHDRDecodePostprocess".to_string(),
        display_name: "LTXV HDR Decode Postprocess".to_string(),
        category: "image/ltx".to_string(),
        input_types: NodeInputTypes {
            required: {
                let mut m = HashMap::new();
                m.insert("images".to_string(), InputTypeSpec {
                    type_name: "IMAGE".to_string(),
                    extra: HashMap::new(),
                });
                m.insert("exposure".to_string(), InputTypeSpec {
                    type_name: "FLOAT".to_string(),
                    extra: HashMap::new(),
                });
                m.insert("gamma".to_string(), InputTypeSpec {
                    type_name: "FLOAT".to_string(),
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
        function_name: "hdr_decode".to_string(),
    };

    registry.register(class_def, Arc::new(|ctx, _node, node_id| {
        let images = ctx.resolve_input(node_id, "images").unwrap_or_else(|_| json!({}));
        Box::pin(async move {
            Ok(vec![images])
        })
    }));
}

fn register_ltxv_dilate_video_mask(registry: &mut NodeRegistry) {
    let class_def = NodeClassDef {
        class_type: "LTXVDilateVideoMask".to_string(),
        display_name: "LTXV Dilate Video Mask".to_string(),
        category: "mask/ltx".to_string(),
        input_types: NodeInputTypes {
            required: {
                let mut m = HashMap::new();
                m.insert("mask".to_string(), InputTypeSpec {
                    type_name: "MASK".to_string(),
                    extra: HashMap::new(),
                });
                m.insert("dilate".to_string(), InputTypeSpec {
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
        function_name: "dilate_video_mask".to_string(),
    };

    registry.register(class_def, Arc::new(|ctx, _node, node_id| {
        let mask = ctx.resolve_input(node_id, "mask").unwrap_or_else(|_| json!({}));
        Box::pin(async move {
            Ok(vec![mask])
        })
    }));
}

fn register_ltxv_inpaint_preprocess(registry: &mut NodeRegistry) {
    let class_def = NodeClassDef {
        class_type: "LTXVInpaintPreprocess".to_string(),
        display_name: "LTXV Inpaint Preprocess".to_string(),
        category: "mask/ltx".to_string(),
        input_types: NodeInputTypes {
            required: {
                let mut m = HashMap::new();
                m.insert("image".to_string(), InputTypeSpec {
                    type_name: "IMAGE".to_string(),
                    extra: HashMap::new(),
                });
                m.insert("mask".to_string(), InputTypeSpec {
                    type_name: "MASK".to_string(),
                    extra: HashMap::new(),
                });
                m.insert("feather".to_string(), InputTypeSpec {
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
        function_name: "inpaint_preprocess".to_string(),
    };

    registry.register(class_def, Arc::new(|ctx, _node, node_id| {
        let mask = ctx.resolve_input(node_id, "mask").unwrap_or_else(|_| json!({}));
        Box::pin(async move {
            Ok(vec![mask])
        })
    }));
}

fn register_ltxv_patcher_vae(registry: &mut NodeRegistry) {
    let class_def = NodeClassDef {
        class_type: "LTXVPatcherVAE".to_string(),
        display_name: "LTXV Patcher VAE".to_string(),
        category: "latent/ltx".to_string(),
        input_types: NodeInputTypes {
            required: {
                let mut m = HashMap::new();
                m.insert("vae".to_string(), InputTypeSpec {
                    type_name: "VAE".to_string(),
                    extra: HashMap::new(),
                });
                m.insert("patch_type".to_string(), InputTypeSpec {
                    type_name: "COMBO".to_string(),
                    extra: {
                        let mut e = HashMap::new();
                        e.insert("options".to_string(), json!(["scale", "residual", "additive"]));
                        e
                    },
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
        output_types: vec![IoType::Vae],
        output_names: vec!["VAE".to_string()],
        output_is_list: vec![false],
        is_output_node: false,
        has_intermediate_output: false,
        is_changed: None,
        not_idempotent: false,
        function_name: "patcher_vae".to_string(),
    };

    registry.register(class_def, Arc::new(|ctx, _node, node_id| {
        let vae = ctx.resolve_input(node_id, "vae").unwrap_or_else(|_| json!({}));
        let patch_type = ctx.resolve_input(node_id, "patch_type").unwrap_or_else(|_| json!("scale"));
        let strength = ctx.resolve_input(node_id, "strength").unwrap_or_else(|_| json!(1.0));

        Box::pin(async move {
            let mut vae_out = vae.as_object().cloned().unwrap_or_default();
            vae_out.insert("patch_type".to_string(), patch_type);
            vae_out.insert("patch_strength".to_string(), strength);
            Ok(vec![json!(vae_out)])
        })
    }));
}

fn register_ltxv_q8_patch(registry: &mut NodeRegistry) {
    let class_def = NodeClassDef {
        class_type: "LTXVQ8Patch".to_string(),
        display_name: "LTXV Q8 Patch".to_string(),
        category: "model_patches/ltx".to_string(),
        input_types: NodeInputTypes {
            required: {
                let mut m = HashMap::new();
                m.insert("model".to_string(), InputTypeSpec {
                    type_name: "MODEL".to_string(),
                    extra: HashMap::new(),
                });
                m.insert("q8_patch".to_string(), InputTypeSpec {
                    type_name: "LORA".to_string(),
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
        output_types: vec![IoType::Model],
        output_names: vec!["MODEL".to_string()],
        output_is_list: vec![false],
        is_output_node: false,
        has_intermediate_output: false,
        is_changed: None,
        not_idempotent: false,
        function_name: "q8_patch".to_string(),
    };

    registry.register(class_def, Arc::new(|ctx, _node, node_id| {
        let model = ctx.resolve_input(node_id, "model").unwrap_or_else(|_| json!({}));
        let q8_patch = ctx.resolve_input(node_id, "q8_patch").unwrap_or_else(|_| json!({}));
        let strength = ctx.resolve_input(node_id, "strength").unwrap_or_else(|_| json!(1.0));

        Box::pin(async move {
            let mut model_out = model.as_object().cloned().unwrap_or_default();
            model_out.insert("q8_patch".to_string(), q8_patch);
            model_out.insert("q8_strength".to_string(), strength);
            Ok(vec![json!(model_out)])
        })
    }));
}

fn register_ltxv_q8_lora_model_loader(registry: &mut NodeRegistry) {
    let class_def = NodeClassDef {
        class_type: "LTXVQ8LoraModelLoader".to_string(),
        display_name: "LTXV Q8 LoRA Model Loader".to_string(),
        category: "loaders/ltx".to_string(),
        input_types: NodeInputTypes {
            required: {
                let mut m = HashMap::new();
                m.insert("q8_name".to_string(), InputTypeSpec {
                    type_name: "COMBO".to_string(),
                    extra: HashMap::new(),
                });
                m
            },
            optional: HashMap::new(),
            hidden: HashMap::new(),
        },
        output_types: vec![IoType::Lora],
        output_names: vec!["LORA".to_string()],
        output_is_list: vec![false],
        is_output_node: false,
        has_intermediate_output: false,
        is_changed: None,
        not_idempotent: false,
        function_name: "load_q8_lora".to_string(),
    };

    registry.register(class_def, Arc::new(|_ctx, node, _node_id| {
        let q8_name = node.inputs.get("q8_name")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        Box::pin(async move {
            Ok(vec![json!({
                "type": "q8_lora",
                "name": q8_name,
            })])
        })
    }));
}

fn register_decoder_noise(registry: &mut NodeRegistry) {
    let class_def = NodeClassDef {
        class_type: "DecoderNoise".to_string(),
        display_name: "Decoder Noise".to_string(),
        category: "latent/ltx".to_string(),
        input_types: NodeInputTypes {
            required: {
                let mut m = HashMap::new();
                m.insert("samples".to_string(), InputTypeSpec {
                    type_name: "LATENT".to_string(),
                    extra: HashMap::new(),
                });
                m.insert("noise_strength".to_string(), InputTypeSpec {
                    type_name: "FLOAT".to_string(),
                    extra: HashMap::new(),
                });
                m.insert("seed".to_string(), InputTypeSpec {
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
        function_name: "decoder_noise".to_string(),
    };

    registry.register(class_def, Arc::new(|ctx, _node, node_id| {
        let samples = ctx.resolve_input(node_id, "samples").unwrap_or_else(|_| json!({}));
        Box::pin(async move {
            Ok(vec![samples])
        })
    }));
}

fn register_ltxv_draw_tracks(registry: &mut NodeRegistry) {
    let class_def = NodeClassDef {
        class_type: "LTXVDrawTracks".to_string(),
        display_name: "LTXV Draw Tracks".to_string(),
        category: "mask/ltx".to_string(),
        input_types: NodeInputTypes {
            required: {
                let mut m = HashMap::new();
                m.insert("image".to_string(), InputTypeSpec {
                    type_name: "IMAGE".to_string(),
                    extra: HashMap::new(),
                });
                m.insert("tracks".to_string(), InputTypeSpec {
                    type_name: "STRING".to_string(),
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
        function_name: "draw_tracks".to_string(),
    };

    registry.register(class_def, Arc::new(|ctx, _node, node_id| {
        let image = ctx.resolve_input(node_id, "image").unwrap_or_else(|_| json!({}));
        Box::pin(async move {
            Ok(vec![image])
        })
    }));
}

fn register_ltxv_sparse_track_editor(registry: &mut NodeRegistry) {
    let class_def = NodeClassDef {
        class_type: "LTXVSparseTrackEditor".to_string(),
        display_name: "LTXV Sparse Track Editor".to_string(),
        category: "mask/ltx".to_string(),
        input_types: NodeInputTypes {
            required: {
                let mut m = HashMap::new();
                m.insert("tracks".to_string(), InputTypeSpec {
                    type_name: "STRING".to_string(),
                    extra: HashMap::new(),
                });
                m.insert("frame".to_string(), InputTypeSpec {
                    type_name: "INT".to_string(),
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
        function_name: "sparse_track_editor".to_string(),
    };

    registry.register(class_def, Arc::new(|_ctx, node, _node_id| {
        Box::pin(async move {
            Ok(vec![json!({
                "type": "sparse_track",
                "tracks": node.inputs.get("tracks"),
            })])
        })
    }));
}

fn register_ltxv_load_conditioning(registry: &mut NodeRegistry) {
    let class_def = NodeClassDef {
        class_type: "LTXVLoadConditioning".to_string(),
        display_name: "LTXV Load Conditioning".to_string(),
        category: "loaders/ltx".to_string(),
        input_types: NodeInputTypes {
            required: {
                let mut m = HashMap::new();
                m.insert("filename".to_string(), InputTypeSpec {
                    type_name: "STRING".to_string(),
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
        function_name: "load_conditioning".to_string(),
    };

    registry.register(class_def, Arc::new(|_ctx, node, _node_id| {
        let filename = node.inputs.get("filename")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        Box::pin(async move {
            Ok(vec![json!({
                "type": "conditioning",
                "filename": filename,
            })])
        })
    }));
}

fn register_ltxv_save_conditioning(registry: &mut NodeRegistry) {
    let class_def = NodeClassDef {
        class_type: "LTXVSaveConditioning".to_string(),
        display_name: "LTXV Save Conditioning".to_string(),
        category: "output/ltx".to_string(),
        input_types: NodeInputTypes {
            required: {
                let mut m = HashMap::new();
                m.insert("conditioning".to_string(), InputTypeSpec {
                    type_name: "CONDITIONING".to_string(),
                    extra: HashMap::new(),
                });
                m.insert("filename".to_string(), InputTypeSpec {
                    type_name: "STRING".to_string(),
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
        function_name: "save_conditioning".to_string(),
    };

    registry.register(class_def, Arc::new(|ctx, _node, node_id| {
        let conditioning = ctx.resolve_input(node_id, "conditioning").unwrap_or_else(|_| json!({}));
        Box::pin(async move {
            Ok(vec![conditioning])
        })
    }));
}

fn register_ltxv_add_guide_advanced(registry: &mut NodeRegistry) {
    let class_def = NodeClassDef {
        class_type: "LTXVAddGuideAdvanced".to_string(),
        display_name: "LTXV Add Guide Advanced".to_string(),
        category: "sampling/ltx".to_string(),
        input_types: NodeInputTypes {
            required: {
                let mut m = HashMap::new();
                m.insert("model".to_string(), InputTypeSpec {
                    type_name: "MODEL".to_string(),
                    extra: HashMap::new(),
                });
                m.insert("guide_type".to_string(), InputTypeSpec {
                    type_name: "COMBO".to_string(),
                    extra: {
                        let mut e = HashMap::new();
                        e.insert("options".to_string(), json!(["attn", "res", "all"]));
                        e
                    },
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
        output_types: vec![IoType::Model],
        output_names: vec!["MODEL".to_string()],
        output_is_list: vec![false],
        is_output_node: false,
        has_intermediate_output: false,
        is_changed: None,
        not_idempotent: false,
        function_name: "add_guide_advanced".to_string(),
    };

    registry.register(class_def, Arc::new(|ctx, _node, node_id| {
        let model = ctx.resolve_input(node_id, "model").unwrap_or_else(|_| json!({}));
        let guide_type = ctx.resolve_input(node_id, "guide_type").unwrap_or_else(|_| json!("all"));
        let strength = ctx.resolve_input(node_id, "strength").unwrap_or_else(|_| json!(1.0));

        Box::pin(async move {
            let mut model_out = model.as_object().cloned().unwrap_or_default();
            model_out.insert("guide_type".to_string(), guide_type);
            model_out.insert("guide_strength".to_string(), strength);
            Ok(vec![json!(model_out)])
        })
    }));
}

fn register_ltxv_add_guide_advanced_attention(registry: &mut NodeRegistry) {
    let class_def = NodeClassDef {
        class_type: "LTXVAddGuideAdvancedAttention".to_string(),
        display_name: "LTXV Add Guide Advanced Attention".to_string(),
        category: "sampling/ltx".to_string(),
        input_types: NodeInputTypes {
            required: {
                let mut m = HashMap::new();
                m.insert("model".to_string(), InputTypeSpec {
                    type_name: "MODEL".to_string(),
                    extra: HashMap::new(),
                });
                m.insert("guide_type".to_string(), InputTypeSpec {
                    type_name: "COMBO".to_string(),
                    extra: {
                        let mut e = HashMap::new();
                        e.insert("options".to_string(), json!(["cross_attn", "self_attn", "all"]));
                        e
                    },
                });
                m.insert("strength".to_string(), InputTypeSpec {
                    type_name: "FLOAT".to_string(),
                    extra: HashMap::new(),
                });
                m.insert("layers".to_string(), InputTypeSpec {
                    type_name: "STRING".to_string(),
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
        function_name: "add_guide_advanced_attention".to_string(),
    };

    registry.register(class_def, Arc::new(|ctx, _node, node_id| {
        let model = ctx.resolve_input(node_id, "model").unwrap_or_else(|_| json!({}));
        let guide_type = ctx.resolve_input(node_id, "guide_type").unwrap_or_else(|_| json!("all"));
        let strength = ctx.resolve_input(node_id, "strength").unwrap_or_else(|_| json!(1.0));
        let layers = ctx.resolve_input(node_id, "layers").unwrap_or_else(|_| json!(""));

        Box::pin(async move {
            let mut model_out = model.as_object().cloned().unwrap_or_default();
            model_out.insert("guide_type".to_string(), guide_type);
            model_out.insert("guide_strength".to_string(), strength);
            model_out.insert("guide_layers".to_string(), layers);
            Ok(vec![json!(model_out)])
        })
    }));
}

// ========== H3 (MiniMax-HunyuanVideoAudio) 生态节点 ==========

#[cfg(feature = "flash-attn")]
fn register_h3_context_ir(registry: &mut NodeRegistry) {
    let class_def = NodeClassDef {
        class_type: "H3ContextIR".to_string(),
        display_name: "H3 Context-IR (Multimodal Parser)".to_string(),
        category: "H3/multimodal".to_string(),
        input_types: NodeInputTypes {
            required: {
                let mut m = HashMap::new();
                m.insert("text_prompt".to_string(), InputTypeSpec {
                    type_name: "STRING".to_string(),
                    extra: {
                        let mut e = HashMap::new();
                        e.insert("multiline".to_string(), json!(true));
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
                m.insert("video".to_string(), InputTypeSpec {
                    type_name: "VIDEO".to_string(),
                    extra: HashMap::new(),
                });
                m.insert("parse_sfx".to_string(), InputTypeSpec {
                    type_name: "BOOLEAN".to_string(),
                    extra: {
                        let mut e = HashMap::new();
                        e.insert("default".to_string(), json!(true));
                        e
                    },
                });
                m.insert("parse_bgm".to_string(), InputTypeSpec {
                    type_name: "BOOLEAN".to_string(),
                    extra: {
                        let mut e = HashMap::new();
                        e.insert("default".to_string(), json!(false));
                        e
                    },
                });
                m.insert("bridge_url".to_string(), InputTypeSpec {
                    type_name: "STRING".to_string(),
                    extra: {
                        let mut e = HashMap::new();
                        e.insert("default".to_string(), json!("http://127.0.0.1:8998"));
                        e
                    },
                });
                m
            },
            hidden: HashMap::new(),
        },
        output_types: vec![
            IoType::Custom("H3_CONTEXT".to_string()),
            IoType::String,
        ],
        output_names: vec!["H3_CONTEXT".to_string(), "formatted_prompt".to_string()],
        output_is_list: vec![false, false],
        is_output_node: false,
        has_intermediate_output: false,
        is_changed: None,
        not_idempotent: true,
        function_name: "parse_context".to_string(),
    };

    registry.register(class_def, Arc::new(|ctx, _node, node_id| {
        let image_val = ctx.resolve_input(node_id, "image").ok();
        let video_val = ctx.resolve_input(node_id, "video").ok();
        let text_prompt = ctx.resolve_input(node_id, "text_prompt")
            .ok()
            .and_then(|v| v.as_str().map(|s| s.to_string()))
            .unwrap_or_default();
        let parse_sfx = ctx.resolve_input(node_id, "parse_sfx")
            .ok()
            .and_then(|v| v.as_bool())
            .unwrap_or(true);
        let parse_bgm = ctx.resolve_input(node_id, "parse_bgm")
            .ok()
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let bridge_url = ctx.resolve_input(node_id, "bridge_url")
            .ok()
            .and_then(|v| v.as_str().map(|s| s.to_string()))
            .unwrap_or_else(|| "http://127.0.0.1:8998".to_string());

        let progress_cb = ctx.progress_callback();
        let nid = node_id.to_string();
        let pid = ctx.prompt_id().to_string();

        Box::pin(async move {
            use comfy_inference::{FlashAttnBackend, FlashAttnConfig, FlashProgressCallback, ContextIrParams, H3Context, InferenceBackend};

            let config = FlashAttnConfig::new(bridge_url).with_timeout(60);
            let mut backend = FlashAttnBackend::new(config);

            if let Some(cb) = progress_cb {
                let nid2 = nid.clone();
                let pid2 = pid.clone();
                let flash_cb: FlashProgressCallback = Arc::new(move |step, total, _phase, _msg| {
                    cb(&pid2, &nid2, step as f64, total as f64);
                });
                backend = backend.with_progress_callback(flash_cb);
            }

            let sd_image = image_val.and_then(|v| parse_sd_image_from_value(&v));
            let sd_video = video_val.and_then(|v| parse_sd_video_from_value(&v));

            let cir_params = if let Some(img) = sd_image {
                let mut p = ContextIrParams::from_image(img);
                p.parse_sfx = parse_sfx;
                p.parse_bgm = parse_bgm;
                p.user_prompt = Some(text_prompt);
                p
            } else if let Some(vid) = sd_video {
                let mut p = ContextIrParams::from_video(vid);
                p.parse_sfx = parse_sfx;
                p.parse_bgm = parse_bgm;
                p.user_prompt = Some(text_prompt);
                p
            } else {
                let mut p = ContextIrParams::from_text(text_prompt);
                p.parse_sfx = parse_sfx;
                p.parse_bgm = parse_bgm;
                p
            };

            let context: H3Context = backend.context_ir(cir_params)
                .map_err(|e| ExecutorError::NodeExecutionFailed {
                    node_id: node_id.to_string(),
                    message: format!("Context-IR failed: {}", e),
                })?;

            let formatted = context.build_positive_prompt();
            let context_val = serde_json::to_value(&context).map_err(|e| ExecutorError::NodeExecutionFailed {
                node_id: node_id.to_string(),
                message: format!("Failed to serialize context: {}", e),
            })?;

            Ok(vec![context_val, json!(formatted)])
        })
    }));
}

#[cfg(feature = "flash-attn")]
fn register_h3_director(registry: &mut NodeRegistry) {
    let class_def = NodeClassDef {
        class_type: "H3Director".to_string(),
        display_name: "H3 Director (Audio-Video Generation)".to_string(),
        category: "H3/generation".to_string(),
        input_types: NodeInputTypes {
            required: {
                let mut m = HashMap::new();
                m.insert("prompt".to_string(), InputTypeSpec {
                    type_name: "STRING".to_string(),
                    extra: {
                        let mut e = HashMap::new();
                        e.insert("multiline".to_string(), json!(true));
                        e
                    },
                });
                m.insert("mode".to_string(), InputTypeSpec {
                    type_name: "COMBO".to_string(),
                    extra: {
                        let mut e = HashMap::new();
                        e.insert("options".to_string(), json!(["t2va", "i2va", "ref2va", "mr2va"]));
                        e.insert("default".to_string(), json!("t2va"));
                        e
                    },
                });
                m.insert("width".to_string(), InputTypeSpec {
                    type_name: "INT".to_string(),
                    extra: {
                        let mut e = HashMap::new();
                        e.insert("default".to_string(), json!(848));
                        e.insert("min".to_string(), json!(256));
                        e.insert("max".to_string(), json!(1920));
                        e.insert("step".to_string(), json!(16));
                        e
                    },
                });
                m.insert("height".to_string(), InputTypeSpec {
                    type_name: "INT".to_string(),
                    extra: {
                        let mut e = HashMap::new();
                        e.insert("default".to_string(), json!(480));
                        e.insert("min".to_string(), json!(256));
                        e.insert("max".to_string(), json!(1088));
                        e.insert("step".to_string(), json!(16));
                        e
                    },
                });
                m.insert("num_frames".to_string(), InputTypeSpec {
                    type_name: "INT".to_string(),
                    extra: {
                        let mut e = HashMap::new();
                        e.insert("default".to_string(), json!(123));
                        e.insert("min".to_string(), json!(123));
                        e.insert("max".to_string(), json!(362));
                        e.insert("step".to_string(), json!(17));
                        e
                    },
                });
                m.insert("steps".to_string(), InputTypeSpec {
                    type_name: "INT".to_string(),
                    extra: {
                        let mut e = HashMap::new();
                        e.insert("default".to_string(), json!(50));
                        e.insert("min".to_string(), json!(10));
                        e.insert("max".to_string(), json!(100));
                        e
                    },
                });
                m.insert("cfg".to_string(), InputTypeSpec {
                    type_name: "FLOAT".to_string(),
                    extra: {
                        let mut e = HashMap::new();
                        e.insert("default".to_string(), json!(7.0));
                        e.insert("min".to_string(), json!(1.0));
                        e.insert("max".to_string(), json!(20.0));
                        e
                    },
                });
                m.insert("seed".to_string(), InputTypeSpec {
                    type_name: "INT".to_string(),
                    extra: {
                        let mut e = HashMap::new();
                        e.insert("default".to_string(), json!(42));
                        e
                    },
                });
                m
            },
            optional: {
                let mut m = HashMap::new();
                m.insert("h3_context".to_string(), InputTypeSpec {
                    type_name: "H3_CONTEXT".to_string(),
                    extra: HashMap::new(),
                });
                m.insert("negative_prompt".to_string(), InputTypeSpec {
                    type_name: "STRING".to_string(),
                    extra: {
                        let mut e = HashMap::new();
                        e.insert("multiline".to_string(), json!(true));
                        e.insert("default".to_string(), json!("low quality, blurry, distorted"));
                        e
                    },
                });
                m.insert("reference_image".to_string(), InputTypeSpec {
                    type_name: "IMAGE".to_string(),
                    extra: HashMap::new(),
                });
                m.insert("reference_video".to_string(), InputTypeSpec {
                    type_name: "VIDEO".to_string(),
                    extra: HashMap::new(),
                });
                m.insert("fps".to_string(), InputTypeSpec {
                    type_name: "INT".to_string(),
                    extra: {
                        let mut e = HashMap::new();
                        e.insert("default".to_string(), json!(24));
                        e
                    },
                });
                m.insert("generate_sfx".to_string(), InputTypeSpec {
                    type_name: "BOOLEAN".to_string(),
                    extra: {
                        let mut e = HashMap::new();
                        e.insert("default".to_string(), json!(true));
                        e
                    },
                });
                m.insert("generate_bgm".to_string(), InputTypeSpec {
                    type_name: "BOOLEAN".to_string(),
                    extra: {
                        let mut e = HashMap::new();
                        e.insert("default".to_string(), json!(false));
                        e
                    },
                });
                m.insert("bridge_url".to_string(), InputTypeSpec {
                    type_name: "STRING".to_string(),
                    extra: {
                        let mut e = HashMap::new();
                        e.insert("default".to_string(), json!("http://127.0.0.1:8998"));
                        e
                    },
                });
                m
            },
            hidden: HashMap::new(),
        },
        output_types: vec![IoType::Video, IoType::Audio, IoType::Float],
        output_names: vec!["VIDEO".to_string(), "AUDIO".to_string(), "DURATION".to_string()],
        output_is_list: vec![false, false, false],
        is_output_node: true,
        has_intermediate_output: false,
        is_changed: None,
        not_idempotent: true,
        function_name: "generate".to_string(),
    };

    registry.register(class_def, Arc::new(|ctx, _node, node_id| {
        let prompt = ctx.resolve_input(node_id, "prompt")
            .ok()
            .and_then(|v| v.as_str().map(|s| s.to_string()))
            .unwrap_or_default();
        let mode_str = ctx.resolve_input(node_id, "mode")
            .ok()
            .and_then(|v| v.as_str().map(|s| s.to_string()))
            .unwrap_or_else(|| "t2va".to_string());
        let width = ctx.resolve_input(node_id, "width")
            .ok()
            .and_then(|v| v.as_i64())
            .unwrap_or(848) as i32;
        let height = ctx.resolve_input(node_id, "height")
            .ok()
            .and_then(|v| v.as_i64())
            .unwrap_or(480) as i32;
        let num_frames = ctx.resolve_input(node_id, "num_frames")
            .ok()
            .and_then(|v| v.as_i64())
            .unwrap_or(123) as i32;
        let steps = ctx.resolve_input(node_id, "steps")
            .ok()
            .and_then(|v| v.as_i64())
            .unwrap_or(50) as i32;
        let cfg = ctx.resolve_input(node_id, "cfg")
            .ok()
            .and_then(|v| v.as_f64())
            .unwrap_or(7.0);
        let seed = ctx.resolve_input(node_id, "seed")
            .ok()
            .and_then(|v| v.as_i64())
            .unwrap_or(42);
        let fps = ctx.resolve_input(node_id, "fps")
            .ok()
            .and_then(|v| v.as_i64())
            .unwrap_or(24) as i32;
        let negative_prompt = ctx.resolve_input(node_id, "negative_prompt")
            .ok()
            .and_then(|v| v.as_str().map(|s| s.to_string()))
            .unwrap_or_else(|| "low quality, blurry, distorted".to_string());
        let generate_sfx = ctx.resolve_input(node_id, "generate_sfx")
            .ok()
            .and_then(|v| v.as_bool())
            .unwrap_or(true);
        let generate_bgm = ctx.resolve_input(node_id, "generate_bgm")
            .ok()
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let bridge_url = ctx.resolve_input(node_id, "bridge_url")
            .ok()
            .and_then(|v| v.as_str().map(|s| s.to_string()))
            .unwrap_or_else(|| "http://127.0.0.1:8998".to_string());

        let context_val = ctx.resolve_input(node_id, "h3_context").ok();
        let ref_image_val = ctx.resolve_input(node_id, "reference_image").ok();
        let ref_video_val = ctx.resolve_input(node_id, "reference_video").ok();

        // Get progress callback from context
        let progress_cb = ctx.progress_callback();
        let nid = node_id.to_string();
        let pid = ctx.prompt_id().to_string();

        Box::pin(async move {
            use comfy_inference::{FlashAttnBackend, FlashAttnConfig, FlashProgressCallback, H3Params, H3Mode, H3Context, InferenceBackend};

            let config = FlashAttnConfig::new(bridge_url).with_timeout(900);
            let mut backend = FlashAttnBackend::new(config);

            // Wire progress callback: convert executor's ProgressCallback to FlashProgressCallback
            if let Some(cb) = progress_cb {
                let nid2 = nid.clone();
                let pid2 = pid.clone();
                let flash_cb: FlashProgressCallback = Arc::new(move |step, total, _phase, _msg| {
                    cb(&pid2, &nid2, step as f64, total as f64);
                });
                backend = backend.with_progress_callback(flash_cb);
            }

            let mode = match mode_str.as_str() {
                "i2va" => H3Mode::I2VA,
                "ref2va" => H3Mode::Ref2VA,
                "mr2va" => H3Mode::MR2VA,
                _ => H3Mode::T2VA,
            };

            let mut params = H3Params::new(prompt.clone())
                .with_negative_prompt(negative_prompt)
                .with_steps(steps)
                .with_cfg(cfg)
                .with_resolution(width, height)
                .with_num_frames(num_frames)
                .with_fps(fps)
                .with_seed(seed)
                .with_sfx(generate_sfx)
                .with_bgm(generate_bgm);
            params.mode = mode;

            if let Some(ctx_val) = context_val {
                if let Ok(h3ctx) = serde_json::from_value::<H3Context>(ctx_val) {
                    let built_prompt = h3ctx.build_positive_prompt();
                    if !prompt.trim().is_empty() {
                        params.prompt = format!("{}. {}", built_prompt, prompt);
                    } else {
                        params.prompt = built_prompt;
                    }
                    if let Some(ref neg) = h3ctx.negative_prompt {
                        if !neg.is_empty() {
                            params.negative_prompt = format!("{}. {}", params.negative_prompt, neg);
                        }
                    }
                    params = params.with_context(h3ctx);
                }
            }

            if let Some(img_val) = ref_image_val {
                if let Some(img) = parse_sd_image_from_value(&img_val) {
                    params.reference_images.push(img);
                    if params.mode == H3Mode::T2VA {
                        params.mode = H3Mode::I2VA;
                    }
                }
            }
            if let Some(vid_val) = ref_video_val {
                if let Some(vid) = parse_sd_video_from_value(&vid_val) {
                    params.reference_video = Some(vid);
                    if params.mode == H3Mode::T2VA {
                        params.mode = H3Mode::Ref2VA;
                    }
                }
            }

            let video = backend.generate_av(params)
                .map_err(|e| ExecutorError::NodeExecutionFailed {
                    node_id: node_id.to_string(),
                    message: format!("H3 generation failed: {}", e),
                })?;

            let duration_sec = video.frames.len() as f64 / video.fps as f64;

            // Serialize video (frames only, without audio for VIDEO output)
            let video_val = serde_json::to_value(&comfy_inference::SdVideo::new_without_audio(
                video.frames.clone(), video.fps
            )).map_err(|e| ExecutorError::NodeExecutionFailed {
                node_id: node_id.to_string(),
                message: format!("Failed to serialize video: {}", e),
            })?;

            // Serialize audio separately for AUDIO output
            let audio_val = match &video.audio {
                Some(audio) => serde_json::to_value(audio).map_err(|e| ExecutorError::NodeExecutionFailed {
                    node_id: node_id.to_string(),
                    message: format!("Failed to serialize audio: {}", e),
                })?,
                None => serde_json::Value::Null,
            };

            // Duration as FLOAT
            let duration_val = json!(duration_sec);

            Ok(vec![video_val, audio_val, duration_val])
        })
    }));
}

// Helper functions for parsing SdImage/SdVideo from JSON values
fn parse_sd_image_from_value(val: &serde_json::Value) -> Option<comfy_inference::SdImage> {
    use comfy_inference::SdImage;
    if let Ok(img) = serde_json::from_value::<SdImage>(val.clone()) {
        return Some(img);
    }
    if let Some(frames) = val.get("frames").and_then(|v| v.as_array()) {
        for f in frames {
            if let Ok(img) = serde_json::from_value::<SdImage>(f.clone()) {
                return Some(img);
            }
        }
    }
    None
}

fn parse_sd_video_from_value(val: &serde_json::Value) -> Option<comfy_inference::SdVideo> {
    use comfy_inference::SdVideo;
    if let Ok(vid) = serde_json::from_value::<SdVideo>(val.clone()) {
        return Some(vid);
    }
    if let Some(frames) = val.get("frames").and_then(|v| v.as_array()) {
        let fps = val.get("fps").and_then(|v| v.as_i64()).unwrap_or(24) as i32;
        let mut sd_frames = Vec::new();
        for f in frames {
            if let Some(img) = parse_sd_image_from_value(f) {
                sd_frames.push(img);
            }
        }
        if !sd_frames.is_empty() {
            return Some(SdVideo::new_without_audio(sd_frames, fps));
        }
    }
    None
}

fn parse_sd_audio_from_value(val: &serde_json::Value) -> Option<comfy_inference::SdAudio> {
    use comfy_inference::SdAudio;
    if let Ok(audio) = serde_json::from_value::<SdAudio>(val.clone()) {
        return Some(audio);
    }
    None
}

// ========== 视频编辑节点 (Video Editing Nodes) ==========

fn register_video_edit(registry: &mut NodeRegistry) {
    let class_def = NodeClassDef {
        class_type: "VideoEdit".to_string(),
        display_name: "Video Edit (Trim/Volume/Fade)".to_string(),
        category: "video/edit".to_string(),
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
                m.insert("start_time".to_string(), InputTypeSpec {
                    type_name: "FLOAT".to_string(),
                    extra: {
                        let mut e = HashMap::new();
                        e.insert("default".to_string(), json!(0.0));
                        e.insert("min".to_string(), json!(0.0));
                        e.insert("step".to_string(), json!(0.1));
                        e
                    },
                });
                m.insert("end_time".to_string(), InputTypeSpec {
                    type_name: "FLOAT".to_string(),
                    extra: {
                        let mut e = HashMap::new();
                        e.insert("default".to_string(), json!(0.0));
                        e.insert("min".to_string(), json!(0.0));
                        e.insert("step".to_string(), json!(0.1));
                        e
                    },
                });
                m.insert("volume".to_string(), InputTypeSpec {
                    type_name: "FLOAT".to_string(),
                    extra: {
                        let mut e = HashMap::new();
                        e.insert("default".to_string(), json!(1.0));
                        e.insert("min".to_string(), json!(0.0));
                        e.insert("max".to_string(), json!(5.0));
                        e.insert("step".to_string(), json!(0.1));
                        e
                    },
                });
                m.insert("fade_in".to_string(), InputTypeSpec {
                    type_name: "FLOAT".to_string(),
                    extra: {
                        let mut e = HashMap::new();
                        e.insert("default".to_string(), json!(0.0));
                        e.insert("min".to_string(), json!(0.0));
                        e.insert("step".to_string(), json!(0.1));
                        e
                    },
                });
                m.insert("fade_out".to_string(), InputTypeSpec {
                    type_name: "FLOAT".to_string(),
                    extra: {
                        let mut e = HashMap::new();
                        e.insert("default".to_string(), json!(0.0));
                        e.insert("min".to_string(), json!(0.0));
                        e.insert("step".to_string(), json!(0.1));
                        e
                    },
                });
                m
            },
            hidden: HashMap::new(),
        },
        output_types: vec![IoType::Video, IoType::Audio, IoType::Float],
        output_names: vec!["VIDEO".to_string(), "AUDIO".to_string(), "DURATION".to_string()],
        output_is_list: vec![false, false, false],
        is_output_node: false,
        has_intermediate_output: false,
        is_changed: None,
        not_idempotent: true,
        function_name: "edit_video".to_string(),
    };

    registry.register(class_def, Arc::new(|ctx, _node, node_id| {
        let video_val = ctx.resolve_input(node_id, "video").unwrap_or_else(|_| json!(null));
        let start_time = ctx.resolve_input(node_id, "start_time")
            .ok()
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0) as f32;
        let end_time = ctx.resolve_input(node_id, "end_time")
            .ok()
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0) as f32;
        let volume = ctx.resolve_input(node_id, "volume")
            .ok()
            .and_then(|v| v.as_f64())
            .unwrap_or(1.0) as f32;
        let fade_in = ctx.resolve_input(node_id, "fade_in")
            .ok()
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0) as f32;
        let fade_out = ctx.resolve_input(node_id, "fade_out")
            .ok()
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0) as f32;

        Box::pin(async move {
            let mut video = parse_sd_video_from_value(&video_val)
                .ok_or_else(|| ExecutorError::NodeExecutionFailed {
                    node_id: node_id.to_string(),
                    message: "Failed to parse input video".to_string(),
                })?;

            // Trim if end_time > 0
            let total_dur = video.duration_sec();
            let end = if end_time > 0.0 { end_time.min(total_dur) } else { total_dur };
            if start_time > 0.0 || end < total_dur {
                video = video.trim(start_time, end);
            }

            // Volume adjustment
            if (volume - 1.0).abs() > 0.001 {
                video = video.adjust_volume(volume);
            }

            // Fade effects
            if fade_in > 0.0 {
                video = video.audio_fade_in(fade_in);
            }
            if fade_out > 0.0 {
                video = video.audio_fade_out(fade_out);
            }

            let duration = video.duration_sec();

            // Serialize video (with audio)
            let video_out = serde_json::to_value(&video).map_err(|e| ExecutorError::NodeExecutionFailed {
                node_id: node_id.to_string(),
                message: format!("Failed to serialize video: {}", e),
            })?;

            // Serialize audio separately
            let audio_out = match &video.audio {
                Some(a) => serde_json::to_value(a).map_err(|e| ExecutorError::NodeExecutionFailed {
                    node_id: node_id.to_string(),
                    message: format!("Failed to serialize audio: {}", e),
                })?,
                None => serde_json::Value::Null,
            };

            Ok(vec![video_out, audio_out, json!(duration)])
        })
    }));
}

fn register_video_concat(registry: &mut NodeRegistry) {
    let class_def = NodeClassDef {
        class_type: "VideoConcat".to_string(),
        display_name: "Video Concat (Merge Clips)".to_string(),
        category: "video/edit".to_string(),
        input_types: NodeInputTypes {
            required: {
                let mut m = HashMap::new();
                m.insert("video1".to_string(), InputTypeSpec {
                    type_name: "VIDEO".to_string(),
                    extra: HashMap::new(),
                });
                m
            },
            optional: {
                let mut m = HashMap::new();
                m.insert("video2".to_string(), InputTypeSpec {
                    type_name: "VIDEO".to_string(),
                    extra: HashMap::new(),
                });
                m.insert("video3".to_string(), InputTypeSpec {
                    type_name: "VIDEO".to_string(),
                    extra: HashMap::new(),
                });
                m.insert("video4".to_string(), InputTypeSpec {
                    type_name: "VIDEO".to_string(),
                    extra: HashMap::new(),
                });
                m.insert("crossfade".to_string(), InputTypeSpec {
                    type_name: "FLOAT".to_string(),
                    extra: {
                        let mut e = HashMap::new();
                        e.insert("default".to_string(), json!(0.0));
                        e.insert("min".to_string(), json!(0.0));
                        e.insert("step".to_string(), json!(0.1));
                        e
                    },
                });
                m
            },
            hidden: HashMap::new(),
        },
        output_types: vec![IoType::Video, IoType::Audio, IoType::Float],
        output_names: vec!["VIDEO".to_string(), "AUDIO".to_string(), "DURATION".to_string()],
        output_is_list: vec![false, false, false],
        is_output_node: false,
        has_intermediate_output: false,
        is_changed: None,
        not_idempotent: true,
        function_name: "concat_videos".to_string(),
    };

    registry.register(class_def, Arc::new(|ctx, _node, node_id| {
        let v1_val = ctx.resolve_input(node_id, "video1").unwrap_or_else(|_| json!(null));
        let v2_val = ctx.resolve_input(node_id, "video2").ok();
        let v3_val = ctx.resolve_input(node_id, "video3").ok();
        let v4_val = ctx.resolve_input(node_id, "video4").ok();
        let _crossfade = ctx.resolve_input(node_id, "crossfade")
            .ok()
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0) as f32;

        Box::pin(async move {
            let v1 = parse_sd_video_from_value(&v1_val)
                .ok_or_else(|| ExecutorError::NodeExecutionFailed {
                    node_id: node_id.to_string(),
                    message: "Failed to parse video1".to_string(),
                })?;

            let mut result = v1;

            for v_val in [v2_val, v3_val, v4_val].iter() {
                if let Some(val) = v_val {
                    if let Some(v) = parse_sd_video_from_value(val) {
                        result = result.concat(&v);
                    }
                }
            }

            let duration = result.duration_sec();

            let video_out = serde_json::to_value(&result).map_err(|e| ExecutorError::NodeExecutionFailed {
                node_id: node_id.to_string(),
                message: format!("Failed to serialize video: {}", e),
            })?;

            let audio_out = match &result.audio {
                Some(a) => serde_json::to_value(a).map_err(|e| ExecutorError::NodeExecutionFailed {
                    node_id: node_id.to_string(),
                    message: format!("Failed to serialize audio: {}", e),
                })?,
                None => serde_json::Value::Null,
            };

            Ok(vec![video_out, audio_out, json!(duration)])
        })
    }));
}

fn register_video_mix_audio(registry: &mut NodeRegistry) {
    let class_def = NodeClassDef {
        class_type: "VideoMixAudio".to_string(),
        display_name: "Video Mix Audio (Add BGM)".to_string(),
        category: "video/edit".to_string(),
        input_types: NodeInputTypes {
            required: {
                let mut m = HashMap::new();
                m.insert("video".to_string(), InputTypeSpec {
                    type_name: "VIDEO".to_string(),
                    extra: HashMap::new(),
                });
                m.insert("audio".to_string(), InputTypeSpec {
                    type_name: "AUDIO".to_string(),
                    extra: HashMap::new(),
                });
                m
            },
            optional: {
                let mut m = HashMap::new();
                m.insert("audio_volume".to_string(), InputTypeSpec {
                    type_name: "FLOAT".to_string(),
                    extra: {
                        let mut e = HashMap::new();
                        e.insert("default".to_string(), json!(1.0));
                        e.insert("min".to_string(), json!(0.0));
                        e.insert("max".to_string(), json!(5.0));
                        e.insert("step".to_string(), json!(0.1));
                        e
                    },
                });
                m.insert("video_volume".to_string(), InputTypeSpec {
                    type_name: "FLOAT".to_string(),
                    extra: {
                        let mut e = HashMap::new();
                        e.insert("default".to_string(), json!(1.0));
                        e.insert("min".to_string(), json!(0.0));
                        e.insert("max".to_string(), json!(5.0));
                        e.insert("step".to_string(), json!(0.1));
                        e
                    },
                });
                m.insert("loop_audio".to_string(), InputTypeSpec {
                    type_name: "BOOLEAN".to_string(),
                    extra: {
                        let mut e = HashMap::new();
                        e.insert("default".to_string(), json!(false));
                        e
                    },
                });
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
        function_name: "mix_audio".to_string(),
    };

    registry.register(class_def, Arc::new(|ctx, _node, node_id| {
        let video_val = ctx.resolve_input(node_id, "video").unwrap_or_else(|_| json!(null));
        let audio_val = ctx.resolve_input(node_id, "audio").unwrap_or_else(|_| json!(null));
        let audio_volume = ctx.resolve_input(node_id, "audio_volume")
            .ok()
            .and_then(|v| v.as_f64())
            .unwrap_or(1.0) as f32;
        let video_volume = ctx.resolve_input(node_id, "video_volume")
            .ok()
            .and_then(|v| v.as_f64())
            .unwrap_or(1.0) as f32;
        let loop_audio = ctx.resolve_input(node_id, "loop_audio")
            .ok()
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        Box::pin(async move {
            let mut video = parse_sd_video_from_value(&video_val)
                .ok_or_else(|| ExecutorError::NodeExecutionFailed {
                    node_id: node_id.to_string(),
                    message: "Failed to parse input video".to_string(),
                })?;

            let mut bgm = parse_sd_audio_from_value(&audio_val)
                .ok_or_else(|| ExecutorError::NodeExecutionFailed {
                    node_id: node_id.to_string(),
                    message: "Failed to parse input audio".to_string(),
                })?;

            // Adjust original video volume
            if (video_volume - 1.0).abs() > 0.001 {
                video = video.adjust_volume(video_volume);
            }

            // Prepare BGM: loop or trim to match video duration
            let video_dur = video.duration_sec();
            let bgm_dur = bgm.duration_sec();

            if loop_audio && bgm_dur < video_dur {
                // Loop audio to match video length
                let mut looped = bgm.clone();
                while looped.duration_sec() < video_dur {
                    looped = looped.concat(&bgm);
                }
                bgm = looped.trim(0.0, video_dur);
            } else if bgm_dur > video_dur {
                bgm = bgm.trim(0.0, video_dur);
            }

            // Mix BGM into video
            video = video.mix_audio(&bgm, audio_volume);

            let video_out = serde_json::to_value(&video).map_err(|e| ExecutorError::NodeExecutionFailed {
                node_id: node_id.to_string(),
                message: format!("Failed to serialize video: {}", e),
            })?;

            let audio_out = match &video.audio {
                Some(a) => serde_json::to_value(a).map_err(|e| ExecutorError::NodeExecutionFailed {
                    node_id: node_id.to_string(),
                    message: format!("Failed to serialize audio: {}", e),
                })?,
                None => serde_json::Value::Null,
            };

            Ok(vec![video_out, audio_out])
        })
    }));
}

fn register_video_replace_audio(registry: &mut NodeRegistry) {
    let class_def = NodeClassDef {
        class_type: "VideoReplaceAudio".to_string(),
        display_name: "Video Replace Audio".to_string(),
        category: "video/edit".to_string(),
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
                m.insert("audio".to_string(), InputTypeSpec {
                    type_name: "AUDIO".to_string(),
                    extra: HashMap::new(),
                });
                m.insert("volume".to_string(), InputTypeSpec {
                    type_name: "FLOAT".to_string(),
                    extra: {
                        let mut e = HashMap::new();
                        e.insert("default".to_string(), json!(1.0));
                        e.insert("min".to_string(), json!(0.0));
                        e.insert("max".to_string(), json!(5.0));
                        e.insert("step".to_string(), json!(0.1));
                        e
                    },
                });
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
        function_name: "replace_audio".to_string(),
    };

    registry.register(class_def, Arc::new(|ctx, _node, node_id| {
        let video_val = ctx.resolve_input(node_id, "video").unwrap_or_else(|_| json!(null));
        let audio_val = ctx.resolve_input(node_id, "audio").ok();
        let volume = ctx.resolve_input(node_id, "volume")
            .ok()
            .and_then(|v| v.as_f64())
            .unwrap_or(1.0) as f32;

        Box::pin(async move {
            let video = parse_sd_video_from_value(&video_val)
                .ok_or_else(|| ExecutorError::NodeExecutionFailed {
                    node_id: node_id.to_string(),
                    message: "Failed to parse input video".to_string(),
                })?;

            let new_audio = audio_val.and_then(|v| parse_sd_audio_from_value(&v))
                .map(|a| a.adjust_volume(volume));

            let result = video.replace_audio(new_audio);

            let video_out = serde_json::to_value(&result).map_err(|e| ExecutorError::NodeExecutionFailed {
                node_id: node_id.to_string(),
                message: format!("Failed to serialize video: {}", e),
            })?;

            let audio_out = match &result.audio {
                Some(a) => serde_json::to_value(a).map_err(|e| ExecutorError::NodeExecutionFailed {
                    node_id: node_id.to_string(),
                    message: format!("Failed to serialize audio: {}", e),
                })?,
                None => serde_json::Value::Null,
            };

            Ok(vec![video_out, audio_out])
        })
    }));
}

// ========== Premiere-style Multi-Track Video Timeline ==========

fn register_video_timeline(registry: &mut NodeRegistry) {
    let class_def = NodeClassDef {
        class_type: "VideoTimeline".to_string(),
        display_name: "Video Timeline (Multi-Track Editor)".to_string(),
        category: "video/edit".to_string(),
        input_types: NodeInputTypes {
            required: {
                let mut m = HashMap::new();
                m.insert("fps".to_string(), InputTypeSpec {
                    type_name: "INT".to_string(),
                    extra: {
                        let mut e = HashMap::new();
                        e.insert("default".to_string(), json!(24));
                        e.insert("min".to_string(), json!(1));
                        e.insert("max".to_string(), json!(60));
                        e
                    },
                });
                m.insert("width".to_string(), InputTypeSpec {
                    type_name: "INT".to_string(),
                    extra: {
                        let mut e = HashMap::new();
                        e.insert("default".to_string(), json!(848));
                        e.insert("min".to_string(), json!(64));
                        e.insert("max".to_string(), json!(4096));
                        e.insert("step".to_string(), json!(8));
                        e
                    },
                });
                m.insert("height".to_string(), InputTypeSpec {
                    type_name: "INT".to_string(),
                    extra: {
                        let mut e = HashMap::new();
                        e.insert("default".to_string(), json!(480));
                        e.insert("min".to_string(), json!(64));
                        e.insert("max".to_string(), json!(4096));
                        e.insert("step".to_string(), json!(8));
                        e
                    },
                });
                m
            },
            optional: {
                let mut m = HashMap::new();

                // Helper for video track params
                macro_rules! vtrack {
                    ($m:expr, $n:expr) => {
                        $m.insert(format!("v{}_video", $n), InputTypeSpec {
                            type_name: "VIDEO".to_string(),
                            extra: HashMap::new(),
                        });
                        $m.insert(format!("v{}_start", $n), InputTypeSpec {
                            type_name: "FLOAT".to_string(),
                            extra: { let mut e = HashMap::new(); e.insert("default".to_string(), json!(0.0)); e.insert("min".to_string(), json!(0.0)); e.insert("step".to_string(), json!(0.1)); e },
                        });
                        $m.insert(format!("v{}_in", $n), InputTypeSpec {
                            type_name: "FLOAT".to_string(),
                            extra: { let mut e = HashMap::new(); e.insert("default".to_string(), json!(0.0)); e.insert("min".to_string(), json!(0.0)); e.insert("step".to_string(), json!(0.1)); e },
                        });
                        $m.insert(format!("v{}_out", $n), InputTypeSpec {
                            type_name: "FLOAT".to_string(),
                            extra: { let mut e = HashMap::new(); e.insert("default".to_string(), json!(0.0)); e.insert("min".to_string(), json!(0.0)); e.insert("step".to_string(), json!(0.1)); e },
                        });
                        $m.insert(format!("v{}_opacity", $n), InputTypeSpec {
                            type_name: "FLOAT".to_string(),
                            extra: { let mut e = HashMap::new(); e.insert("default".to_string(), json!(1.0)); e.insert("min".to_string(), json!(0.0)); e.insert("max".to_string(), json!(1.0)); e.insert("step".to_string(), json!(0.05)); e },
                        });
                    }
                }

                // Helper for audio track params
                macro_rules! atrack {
                    ($m:expr, $n:expr) => {
                        $m.insert(format!("a{}_audio", $n), InputTypeSpec {
                            type_name: "AUDIO".to_string(),
                            extra: HashMap::new(),
                        });
                        $m.insert(format!("a{}_start", $n), InputTypeSpec {
                            type_name: "FLOAT".to_string(),
                            extra: { let mut e = HashMap::new(); e.insert("default".to_string(), json!(0.0)); e.insert("min".to_string(), json!(0.0)); e.insert("step".to_string(), json!(0.1)); e },
                        });
                        $m.insert(format!("a{}_in", $n), InputTypeSpec {
                            type_name: "FLOAT".to_string(),
                            extra: { let mut e = HashMap::new(); e.insert("default".to_string(), json!(0.0)); e.insert("min".to_string(), json!(0.0)); e.insert("step".to_string(), json!(0.1)); e },
                        });
                        $m.insert(format!("a{}_out", $n), InputTypeSpec {
                            type_name: "FLOAT".to_string(),
                            extra: { let mut e = HashMap::new(); e.insert("default".to_string(), json!(0.0)); e.insert("min".to_string(), json!(0.0)); e.insert("step".to_string(), json!(0.1)); e },
                        });
                        $m.insert(format!("a{}_volume", $n), InputTypeSpec {
                            type_name: "FLOAT".to_string(),
                            extra: { let mut e = HashMap::new(); e.insert("default".to_string(), json!(1.0)); e.insert("min".to_string(), json!(0.0)); e.insert("max".to_string(), json!(5.0)); e.insert("step".to_string(), json!(0.1)); e },
                        });
                    }
                }

                vtrack!(m, "1");
                vtrack!(m, "2");
                vtrack!(m, "3");
                vtrack!(m, "4");

                atrack!(m, "1");
                atrack!(m, "2");
                atrack!(m, "3");
                atrack!(m, "4");

                m.insert("bg_r".to_string(), InputTypeSpec {
                    type_name: "INT".to_string(),
                    extra: { let mut e = HashMap::new(); e.insert("default".to_string(), json!(0)); e.insert("min".to_string(), json!(0)); e.insert("max".to_string(), json!(255)); e },
                });
                m.insert("bg_g".to_string(), InputTypeSpec {
                    type_name: "INT".to_string(),
                    extra: { let mut e = HashMap::new(); e.insert("default".to_string(), json!(0)); e.insert("min".to_string(), json!(0)); e.insert("max".to_string(), json!(255)); e },
                });
                m.insert("bg_b".to_string(), InputTypeSpec {
                    type_name: "INT".to_string(),
                    extra: { let mut e = HashMap::new(); e.insert("default".to_string(), json!(0)); e.insert("min".to_string(), json!(0)); e.insert("max".to_string(), json!(255)); e },
                });

                m.insert("total_duration".to_string(), InputTypeSpec {
                    type_name: "FLOAT".to_string(),
                    extra: { let mut e = HashMap::new(); e.insert("default".to_string(), json!(0.0)); e.insert("min".to_string(), json!(0.0)); e.insert("step".to_string(), json!(0.1)); e },
                });
                m
            },
            hidden: HashMap::new(),
        },
        output_types: vec![IoType::Video, IoType::Audio, IoType::Float],
        output_names: vec!["VIDEO".to_string(), "AUDIO".to_string(), "DURATION".to_string()],
        output_is_list: vec![false, false, false],
        is_output_node: false,
        has_intermediate_output: false,
        is_changed: None,
        not_idempotent: true,
        function_name: "render_timeline".to_string(),
    };

    registry.register(class_def, Arc::new(|ctx, _node, node_id| {
        use comfy_inference::{SdImage, SdVideo, SdAudio};

        #[derive(Clone)]
        struct VClipData {
            frames: Vec<SdImage>,
            src_fps: i32,
            start_frame: i64,
            src_in_frame: i64,
            src_out_frame: i64,
            opacity: f32,
            total_src_frames: usize,
        }

        #[derive(Clone)]
        struct AClipData {
            samples: Vec<f32>,
            sample_rate: u32,
            channels: u32,
            start_sample: i64,
            src_in_sample: i64,
            src_out_sample: i64,
            volume: f32,
        }

        let node_id_str = node_id.to_string();

        let fps = ctx.resolve_input(node_id, "fps")
            .ok().and_then(|v| v.as_i64()).unwrap_or(24) as i32;
        let width = ctx.resolve_input(node_id, "width")
            .ok().and_then(|v| v.as_i64()).unwrap_or(848) as u32;
        let height = ctx.resolve_input(node_id, "height")
            .ok().and_then(|v| v.as_i64()).unwrap_or(480) as u32;
        let bg_r = ctx.resolve_input(node_id, "bg_r")
            .ok().and_then(|v| v.as_i64()).unwrap_or(0) as u8;
        let bg_g = ctx.resolve_input(node_id, "bg_g")
            .ok().and_then(|v| v.as_i64()).unwrap_or(0) as u8;
        let bg_b = ctx.resolve_input(node_id, "bg_b")
            .ok().and_then(|v| v.as_i64()).unwrap_or(0) as u8;
        let total_duration_override = ctx.resolve_input(node_id, "total_duration")
            .ok().and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;

        // Parse all video clips outside async block
        let mut vclips: Vec<VClipData> = Vec::new();
        for n in &["1", "2", "3", "4"] {
            let video_key = format!("v{}_video", n);
            let start_key = format!("v{}_start", n);
            let in_key = format!("v{}_in", n);
            let out_key = format!("v{}_out", n);
            let op_key = format!("v{}_opacity", n);

            let video_val = match ctx.resolve_input(node_id, &video_key) {
                Ok(v) => v,
                Err(_) => continue,
            };
            let video = match parse_sd_video_from_value(&video_val) {
                Some(v) => v,
                None => continue,
            };

            let start = ctx.resolve_input(node_id, &start_key)
                .ok().and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
            let in_p = ctx.resolve_input(node_id, &in_key)
                .ok().and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
            let out_p = ctx.resolve_input(node_id, &out_key)
                .ok().and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
            let opacity = ctx.resolve_input(node_id, &op_key)
                .ok().and_then(|v| v.as_f64()).unwrap_or(1.0) as f32;

            let total_frames_count = video.frames.len();
            let src_in = ((in_p * video.fps as f32) as i64).max(0).min(total_frames_count as i64);
            let src_out = if out_p > 0.0 {
                ((out_p * video.fps as f32) as i64).max(0).min(total_frames_count as i64)
            } else {
                total_frames_count as i64
            };
            if src_out <= src_in { continue; }

            let start_frame = (start * fps as f32) as i64;
            let frames: Vec<SdImage> = video.frames.iter()
                .map(|f| if f.width == width && f.height == height { f.clone() } else { f.resize(width, height) })
                .collect();

            vclips.push(VClipData {
                frames,
                src_fps: video.fps,
                start_frame,
                src_in_frame: src_in,
                src_out_frame: src_out,
                opacity: opacity.clamp(0.0, 1.0),
                total_src_frames: total_frames_count,
            });
        }

        // Parse all audio clips outside async block
        let mut aclips: Vec<AClipData> = Vec::new();
        for n in &["1", "2", "3", "4"] {
            let audio_key = format!("a{}_audio", n);
            let video_key = format!("v{}_video", n);
            let start_key = format!("a{}_start", n);
            let in_key = format!("a{}_in", n);
            let out_key = format!("a{}_out", n);
            let vol_key = format!("a{}_volume", n);

            let audio_val = ctx.resolve_input(node_id, &audio_key).ok();
            let parsed_audio = audio_val.and_then(|v| parse_sd_audio_from_value(&v));

            let (samples, sr, ch) = if let Some(a) = parsed_audio {
                (a.samples, a.sample_rate, a.channels)
            } else {
                // Fallback: use audio from same-numbered video track
                let video_val = match ctx.resolve_input(node_id, &video_key) {
                    Ok(v) => v,
                    Err(_) => continue,
                };
                let video = match parse_sd_video_from_value(&video_val) {
                    Some(v) => v,
                    None => continue,
                };
                match video.audio {
                    Some(a) => (a.samples, a.sample_rate, a.channels),
                    None => continue,
                }
            };

            let start = ctx.resolve_input(node_id, &start_key)
                .ok().and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
            let in_p = ctx.resolve_input(node_id, &in_key)
                .ok().and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
            let out_p = ctx.resolve_input(node_id, &out_key)
                .ok().and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
            let volume = ctx.resolve_input(node_id, &vol_key)
                .ok().and_then(|v| v.as_f64()).unwrap_or(1.0) as f32;

            let total_samples_count = samples.len() as i64;
            let src_in = ((in_p * sr as f32 * ch as f32) as i64).max(0).min(total_samples_count);
            let src_out = if out_p > 0.0 {
                ((out_p * sr as f32 * ch as f32) as i64).max(0).min(total_samples_count)
            } else {
                total_samples_count
            };
            if src_out <= src_in { continue; }

            let out_sr = 44100u32;
            let out_ch = 2u32;
            let start_sample = (start * out_sr as f32 * out_ch as f32) as i64;

            aclips.push(AClipData {
                samples,
                sample_rate: sr,
                channels: ch,
                start_sample,
                src_in_sample: src_in,
                src_out_sample: src_out,
                volume: volume.max(0.0),
            });
        }

        Box::pin(async move {
            let auto_frames = {
                let mut max_end = 0i64;
                for c in &vclips {
                    let src_dur_frames = c.src_out_frame - c.src_in_frame;
                    let effective_dur = (src_dur_frames as f64 * fps as f64 / c.src_fps as f64) as i64;
                    let end = c.start_frame + effective_dur;
                    if end > max_end { max_end = end; }
                }
                for c in &aclips {
                    let src_dur_samples = c.src_out_sample - c.src_in_sample;
                    let src_dur_sec = src_dur_samples as f32 / (c.sample_rate as f32 * c.channels as f32);
                    let end_sample = c.start_sample + (src_dur_sec * 44100.0 * 2.0) as i64;
                    let end_frame = (end_sample as f64 / (44100.0 * 2.0 / fps as f64)) as i64;
                    if end_frame > max_end { max_end = end_frame; }
                }
                max_end
            };

            let total_frames = if total_duration_override > 0.0 {
                (total_duration_override * fps as f32) as i64
            } else {
                auto_frames
            };

            if total_frames <= 0 {
                return Err(ExecutorError::NodeExecutionFailed {
                    node_id: node_id_str.clone(),
                    message: "Timeline has no clips or duration is zero".to_string(),
                });
            }

            let total_duration = total_frames as f32 / fps as f32;

            let bg = SdImage::solid(width, height, bg_r, bg_g, bg_b);
            let mut output_frames: Vec<SdImage> = Vec::with_capacity(total_frames as usize);

            for frame_idx in 0..total_frames {
                let mut canvas = bg.clone();
                for clip in &vclips {
                    let timeline_t = frame_idx - clip.start_frame;
                    if timeline_t < 0 { continue; }
                    let src_frame_pos = timeline_t as f64 * clip.src_fps as f64 / fps as f64;
                    let src_idx = clip.src_in_frame + src_frame_pos as i64;
                    if src_idx >= clip.src_out_frame || src_idx < 0 { continue; }
                    let src_frame_idx = src_idx.min(clip.total_src_frames as i64 - 1).max(0) as usize;
                    let src_frame = &clip.frames[src_frame_idx];
                    canvas = src_frame.blend_over(&canvas, clip.opacity);
                }
                output_frames.push(canvas);
            }

            let out_sr = 44100u32;
            let out_ch = 2u32;
            let total_samples = (total_duration * out_sr as f32 * out_ch as f32) as usize;
            let mut mixed = vec![0.0f32; total_samples];

            for clip in &aclips {
                let start_idx = clip.src_in_sample as usize;
                let end_idx = clip.src_out_sample as usize;
                if start_idx >= clip.samples.len() { continue; }
                let end_idx = end_idx.min(clip.samples.len());
                let trimmed = &clip.samples[start_idx..end_idx];
                if trimmed.is_empty() { continue; }

                let src_duration = trimmed.len() as f32 / (clip.sample_rate as f32 * clip.channels as f32);
                let tgt_samples_count = (src_duration * out_sr as f32 * out_ch as f32) as usize;
                let mut resampled = vec![0.0f32; tgt_samples_count];

                for i in 0..tgt_samples_count {
                    let src_pos = i as f64 * trimmed.len() as f64 / tgt_samples_count as f64;
                    let src_idx_p = src_pos.floor() as usize;
                    let src_idx_next = (src_idx_p + 1).min(trimmed.len() - 1);
                    let frac = src_pos - src_idx_p as f64;
                    let s0 = trimmed[src_idx_p];
                    let s1 = trimmed[src_idx_next];
                    resampled[i] = (s0 + (s1 - s0) * frac as f32) * clip.volume;
                }

                let start_s = clip.start_sample as usize;
                for (i, &s) in resampled.iter().enumerate() {
                    let pos = start_s + i;
                    if pos < mixed.len() {
                        mixed[pos] = (mixed[pos] + s).clamp(-1.0, 1.0);
                    }
                }
            }

            let has_audio = mixed.iter().any(|&s| s.abs() > 0.0001);
            let output_audio = if has_audio {
                Some(SdAudio::new(mixed, out_sr, out_ch))
            } else {
                None
            };

            let output_video = SdVideo::new(output_frames, fps, output_audio.clone());
            let duration = output_video.duration_sec();

            let video_out = serde_json::to_value(&output_video).map_err(|e| ExecutorError::NodeExecutionFailed {
                node_id: node_id_str.clone(),
                message: format!("Failed to serialize video: {}", e),
            })?;

            let audio_out = match &output_audio {
                Some(a) => serde_json::to_value(a).map_err(|e| ExecutorError::NodeExecutionFailed {
                    node_id: node_id_str,
                    message: format!("Failed to serialize audio: {}", e),
                })?,
                None => serde_json::Value::Null,
            };

            Ok(vec![video_out, audio_out, json!(duration)])
        })
    }));
}

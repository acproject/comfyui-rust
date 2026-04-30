use crate::error::ExecutorError;
use crate::registry::NodeRegistry;
use comfy_core::{IoType, NodeClassDef, NodeInputTypes, InputTypeSpec};
use comfy_inference::{ImageGenParams, SampleMethod, Scheduler};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::Arc;

pub fn register_builtin_nodes(registry: &mut NodeRegistry) {
    register_checkpoint_loader(registry);
    register_clip_text_encode(registry);
    register_ksampler(registry);
    register_save_image(registry);
    register_empty_latent_image(registry);
    register_vae_decode(registry);
    register_load_image(registry);
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

        Box::pin(async move {
            Ok(vec![
                json!({ "type": "model", "path": ckpt_name }),
                json!({ "type": "clip", "path": ckpt_name }),
                json!({ "type": "vae", "path": ckpt_name }),
            ])
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
            optional: HashMap::new(),
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
        let model = ctx.resolve_input(node_id, "model").unwrap_or_else(|_| json!(null));
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

        let backend = ctx.backend();
        let supports_img_gen = backend.supports_image_generation();

        Box::pin(async move {
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

                let mut params = ImageGenParams::new(prompt_text)
                    .with_negative_prompt(neg_prompt_text)
                    .with_seed(seed.as_i64().unwrap_or(42))
                    .with_sample_steps(steps.as_i64().unwrap_or(20) as i32)
                    .with_cfg_scale(cfg.as_f64().unwrap_or(7.0) as f32)
                    .with_sample_method(sample_method)
                    .with_scheduler(sched);

                if let Some(latent) = latent_image.as_object() {
                    if let Some(w) = latent.get("width").and_then(|v| v.as_i64()) {
                        if let Some(h) = latent.get("height").and_then(|v| v.as_i64()) {
                            params = params.with_dimensions(w as i32, h as i32);
                        }
                    }
                }

                match backend.generate_image(params) {
                    Ok(images) => {
                        let image_data: Vec<Value> = images.iter().map(|img| {
                            json!({
                                "type": "image",
                                "width": img.width,
                                "height": img.height,
                                "channel": img.channel,
                                "data_len": img.data.len(),
                            })
                        }).collect();
                        Ok(vec![json!({
                            "type": "latent",
                            "samples": image_data,
                            "seed": seed,
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

        Box::pin(async move {
            tracing::info!(
                "SaveImage: saving image with prefix {}",
                filename_prefix.as_str().unwrap_or("ComfyUI")
            );
            Ok(vec![images.clone()])
        })
    }));
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
        let vae = ctx.resolve_input(node_id, "vae")
            .unwrap_or_else(|_| json!(null));

        Box::pin(async move {
            Ok(vec![json!({
                "type": "image",
                "source": "vae_decode",
                "latent": samples,
                "vae": vae,
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

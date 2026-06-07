//! PromptRelay node definitions for ComfyUI-Rust.
//!
//! Implements 5 nodes:
//! - PromptRelayEncode: basic encoding with local prompts
//! - PromptRelayEncodeTimeline: timeline variant with visual editor
//! - PromptRelaySmartEncode: smart syntax parsing
//! - PromptRelaySmartEncodeTest: smart syntax test/debug
//! - PromptRelayAdvancedOptions: advanced relay options

use std::collections::HashMap;
use std::sync::Arc;

use comfy_core::{InputTypeSpec, IoType, NodeClassDef, NodeInputTypes};
use crate::registry::NodeRegistry;
use serde_json::json;

use super::parser;

/// Register all PromptRelay nodes.
pub fn register_prompt_relay_nodes(registry: &mut NodeRegistry) {
    register_prompt_relay_encode(registry);
    register_prompt_relay_encode_timeline(registry);
    register_prompt_relay_smart_encode(registry);
    register_prompt_relay_smart_encode_test(registry);
    register_prompt_relay_advanced_options(registry);

    // WhatDreamsCost nodes
    register_ltx_keyframer(registry);
    register_ltx_sequencer(registry);
    register_ltx_director(registry);
    register_ltx_director_guide(registry);
    register_multi_image_loader(registry);
    register_speech_length_calculator(registry);
    register_load_audio_ui(registry);
    register_load_video_ui(registry);
}

/// PromptRelayEncode - Basic encoding node.
///
/// Encodes global and local prompts with temporal penalty masks for video generation.
fn register_prompt_relay_encode(registry: &mut NodeRegistry) {
    let class_def = NodeClassDef {
        class_type: "PromptRelayEncode".to_string(),
        display_name: "Prompt Relay Encode".to_string(),
        category: "conditioning/prompt_relay".to_string(),
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
                m.insert("latent".to_string(), InputTypeSpec {
                    type_name: "LATENT".to_string(),
                    extra: HashMap::new(),
                });
                m.insert("global_prompt".to_string(), InputTypeSpec {
                    type_name: "STRING".to_string(),
                    extra: {
                        let mut e = HashMap::new();
                        e.insert("multiline".to_string(), json!(true));
                        e
                    },
                });
                m.insert("local_prompts".to_string(), InputTypeSpec {
                    type_name: "STRING".to_string(),
                    extra: {
                        let mut e = HashMap::new();
                        e.insert("multiline".to_string(), json!(true));
                        e
                    },
                });
                m.insert("segment_lengths".to_string(), InputTypeSpec {
                    type_name: "STRING".to_string(),
                    extra: {
                        let mut e = HashMap::new();
                        e.insert("placeholder".to_string(), json!("e.g. 33,33,33 (leave empty for equal split)"));
                        e
                    },
                });
                m.insert("epsilon".to_string(), InputTypeSpec {
                    type_name: "FLOAT".to_string(),
                    extra: {
                        let mut e = HashMap::new();
                        e.insert("default".to_string(), json!(0.001));
                        e.insert("min".to_string(), json!(0.000001));
                        e.insert("max".to_string(), json!(0.99));
                        e.insert("step".to_string(), json!(0.0001));
                        e
                    },
                });
                m
            },
            optional: {
                let mut m = HashMap::new();
                m.insert("relay_options".to_string(), InputTypeSpec {
                    type_name: "RELAY_OPTIONS".to_string(),
                    extra: HashMap::new(),
                });
                m
            },
            hidden: HashMap::new(),
        },
        output_types: vec![IoType::Model, IoType::Conditioning],
        output_names: vec!["MODEL".to_string(), "CONDITIONING".to_string()],
        output_is_list: vec![false, false],
        is_output_node: false,
        has_intermediate_output: false,
        is_changed: None,
        not_idempotent: false,
        function_name: "encode".to_string(),
    };

    registry.register(class_def, Arc::new(|ctx, _node, node_id| {
        let model = ctx.resolve_input(node_id, "model")
            .unwrap_or_else(|_| json!({}));
        let clip = ctx.resolve_input(node_id, "clip")
            .unwrap_or_else(|_| json!(null));
        let latent = ctx.resolve_input(node_id, "latent")
            .unwrap_or_else(|_| json!({}));
        let global_prompt = ctx.resolve_input(node_id, "global_prompt")
            .unwrap_or_else(|_| json!(""))
            .as_str()
            .unwrap_or("")
            .to_string();
        let local_prompts = ctx.resolve_input(node_id, "local_prompts")
            .unwrap_or_else(|_| json!(""))
            .as_str()
            .unwrap_or("")
            .to_string();
        let segment_lengths = ctx.resolve_input(node_id, "segment_lengths")
            .unwrap_or_else(|_| json!(""))
            .as_str()
            .unwrap_or("")
            .to_string();
        let epsilon = ctx.resolve_input(node_id, "epsilon")
            .unwrap_or_else(|_| json!(0.001))
            .as_f64()
            .unwrap_or(0.001) as f32;
        let relay_options = ctx.resolve_input(node_id, "relay_options")
            .ok();

        Box::pin(async move {
            let mut model_out = model.as_object().cloned().unwrap_or_default();

            // Build prompt relay configuration
            let mut relay_config = json!({
                "global_prompt": global_prompt,
                "local_prompts": local_prompts,
                "segment_lengths": segment_lengths,
                "epsilon": epsilon,
            });

            // Merge relay_options if provided
            if let Some(opts) = relay_options {
                if let Some(opts_obj) = opts.as_object() {
                    if let Some(config_obj) = relay_config.as_object_mut() {
                        for (k, v) in opts_obj {
                            config_obj.insert(k.clone(), v.clone());
                        }
                    }
                }
            }

            // Extract latent frame count for validation
            if let Some(latent_obj) = latent.as_object() {
                if let Some(samples) = latent_obj.get("samples") {
                    if let Some(samples_obj) = samples.as_object() {
                        if let Some(frames) = samples_obj.get("length") {
                            if let Some(config_obj) = relay_config.as_object_mut() {
                                config_obj.insert("latent_frames".to_string(), frames.clone());
                            }
                        }
                    }
                }
            }

            model_out.insert("prompt_relay".to_string(), relay_config);

            // Build conditioning output
            let local_parts: Vec<&str> = local_prompts.split('|').map(|s| s.trim()).filter(|s| !s.is_empty()).collect();
            let combined_text = if local_parts.is_empty() {
                global_prompt.clone()
            } else {
                format!("{} {}", global_prompt, local_parts.join(" "))
            };

            let conditioning = json!({
                "type": "conditioning",
                "text": combined_text,
                "clip": clip,
                "prompt_relay": {
                    "global_prompt": global_prompt,
                    "local_prompts": local_prompts,
                    "segment_lengths": segment_lengths,
                    "epsilon": epsilon,
                },
            });

            Ok(vec![json!(model_out), conditioning])
        })
    }));
}

/// PromptRelayEncodeTimeline - Timeline variant with visual editor.
///
/// Same as PromptRelayEncode but with timeline editor support.
fn register_prompt_relay_encode_timeline(registry: &mut NodeRegistry) {
    let class_def = NodeClassDef {
        class_type: "PromptRelayEncodeTimeline".to_string(),
        display_name: "Prompt Relay Encode (Timeline)".to_string(),
        category: "conditioning/prompt_relay".to_string(),
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
                m.insert("latent".to_string(), InputTypeSpec {
                    type_name: "LATENT".to_string(),
                    extra: HashMap::new(),
                });
                m.insert("global_prompt".to_string(), InputTypeSpec {
                    type_name: "STRING".to_string(),
                    extra: {
                        let mut e = HashMap::new();
                        e.insert("multiline".to_string(), json!(true));
                        e
                    },
                });
                m.insert("max_frames".to_string(), InputTypeSpec {
                    type_name: "INT".to_string(),
                    extra: {
                        let mut e = HashMap::new();
                        e.insert("default".to_string(), json!(129));
                        e.insert("min".to_string(), json!(1));
                        e
                    },
                });
                m.insert("local_prompts".to_string(), InputTypeSpec {
                    type_name: "STRING".to_string(),
                    extra: {
                        let mut e = HashMap::new();
                        e.insert("multiline".to_string(), json!(true));
                        e.insert("timelineHidden".to_string(), json!(true));
                        e
                    },
                });
                m.insert("segment_lengths".to_string(), InputTypeSpec {
                    type_name: "STRING".to_string(),
                    extra: {
                        let mut e = HashMap::new();
                        e.insert("timelineHidden".to_string(), json!(true));
                        e
                    },
                });
                m.insert("timeline_data".to_string(), InputTypeSpec {
                    type_name: "STRING".to_string(),
                    extra: {
                        let mut e = HashMap::new();
                        e.insert("timelineHidden".to_string(), json!(true));
                        e.insert("default".to_string(), json!(""));
                        e
                    },
                });
                m.insert("fps".to_string(), InputTypeSpec {
                    type_name: "FLOAT".to_string(),
                    extra: {
                        let mut e = HashMap::new();
                        e.insert("default".to_string(), json!(24.0));
                        e
                    },
                });
                m.insert("time_units".to_string(), InputTypeSpec {
                    type_name: "COMBO".to_string(),
                    extra: {
                        let mut e = HashMap::new();
                        e.insert("choices".to_string(), json!(["frames", "seconds"]));
                        e.insert("default".to_string(), json!(0));
                        e
                    },
                });
                m.insert("epsilon".to_string(), InputTypeSpec {
                    type_name: "FLOAT".to_string(),
                    extra: {
                        let mut e = HashMap::new();
                        e.insert("default".to_string(), json!(0.001));
                        e.insert("min".to_string(), json!(0.000001));
                        e.insert("max".to_string(), json!(0.99));
                        e.insert("step".to_string(), json!(0.0001));
                        e
                    },
                });
                m
            },
            optional: {
                let mut m = HashMap::new();
                m.insert("relay_options".to_string(), InputTypeSpec {
                    type_name: "RELAY_OPTIONS".to_string(),
                    extra: HashMap::new(),
                });
                m
            },
            hidden: HashMap::new(),
        },
        output_types: vec![IoType::Model, IoType::Conditioning],
        output_names: vec!["MODEL".to_string(), "CONDITIONING".to_string()],
        output_is_list: vec![false, false],
        is_output_node: false,
        has_intermediate_output: false,
        is_changed: None,
        not_idempotent: false,
        function_name: "encode".to_string(),
    };

    registry.register(class_def, Arc::new(|ctx, _node, node_id| {
        let model = ctx.resolve_input(node_id, "model")
            .unwrap_or_else(|_| json!({}));
        let clip = ctx.resolve_input(node_id, "clip")
            .unwrap_or_else(|_| json!(null));
        let latent = ctx.resolve_input(node_id, "latent")
            .unwrap_or_else(|_| json!({}));
        let global_prompt = ctx.resolve_input(node_id, "global_prompt")
            .unwrap_or_else(|_| json!(""))
            .as_str()
            .unwrap_or("")
            .to_string();
        let max_frames = ctx.resolve_input(node_id, "max_frames")
            .unwrap_or_else(|_| json!(129))
            .as_i64()
            .unwrap_or(129) as i32;
        let local_prompts = ctx.resolve_input(node_id, "local_prompts")
            .unwrap_or_else(|_| json!(""))
            .as_str()
            .unwrap_or("")
            .to_string();
        let segment_lengths = ctx.resolve_input(node_id, "segment_lengths")
            .unwrap_or_else(|_| json!(""))
            .as_str()
            .unwrap_or("")
            .to_string();
        let timeline_data = ctx.resolve_input(node_id, "timeline_data")
            .unwrap_or_else(|_| json!(""))
            .as_str()
            .unwrap_or("")
            .to_string();
        let fps = ctx.resolve_input(node_id, "fps")
            .unwrap_or_else(|_| json!(24.0))
            .as_f64()
            .unwrap_or(24.0) as f32;
        let time_units = ctx.resolve_input(node_id, "time_units")
            .unwrap_or_else(|_| json!("frames"))
            .as_str()
            .unwrap_or("frames")
            .to_string();
        let epsilon = ctx.resolve_input(node_id, "epsilon")
            .unwrap_or_else(|_| json!(0.001))
            .as_f64()
            .unwrap_or(0.001) as f32;
        let relay_options = ctx.resolve_input(node_id, "relay_options")
            .ok();

        Box::pin(async move {
            let mut model_out = model.as_object().cloned().unwrap_or_default();

            let mut relay_config = json!({
                "global_prompt": global_prompt,
                "local_prompts": local_prompts,
                "segment_lengths": segment_lengths,
                "epsilon": epsilon,
                "max_frames": max_frames,
                "timeline_data": timeline_data,
                "fps": fps,
                "time_units": time_units,
            });

            if let Some(opts) = relay_options {
                if let Some(opts_obj) = opts.as_object() {
                    if let Some(config_obj) = relay_config.as_object_mut() {
                        for (k, v) in opts_obj {
                            config_obj.insert(k.clone(), v.clone());
                        }
                    }
                }
            }

            if let Some(latent_obj) = latent.as_object() {
                if let Some(samples) = latent_obj.get("samples") {
                    if let Some(samples_obj) = samples.as_object() {
                        if let Some(frames) = samples_obj.get("length") {
                            if let Some(config_obj) = relay_config.as_object_mut() {
                                config_obj.insert("latent_frames".to_string(), frames.clone());
                            }
                        }
                    }
                }
            }

            model_out.insert("prompt_relay".to_string(), relay_config);

            let local_parts: Vec<&str> = local_prompts.split('|').map(|s| s.trim()).filter(|s| !s.is_empty()).collect();
            let combined_text = if local_parts.is_empty() {
                global_prompt.clone()
            } else {
                format!("{} {}", global_prompt, local_parts.join(" "))
            };

            let conditioning = json!({
                "type": "conditioning",
                "text": combined_text,
                "clip": clip,
                "prompt_relay": {
                    "global_prompt": global_prompt,
                    "local_prompts": local_prompts,
                    "segment_lengths": segment_lengths,
                    "epsilon": epsilon,
                },
            });

            Ok(vec![json!(model_out), conditioning])
        })
    }));
}

/// PromptRelaySmartEncode - Smart syntax encoding node.
///
/// Parses smart prompt syntax (inline or block) and converts to relay encoding.
fn register_prompt_relay_smart_encode(registry: &mut NodeRegistry) {
    let class_def = NodeClassDef {
        class_type: "PromptRelaySmartEncode".to_string(),
        display_name: "Prompt Relay Encode (Smart)".to_string(),
        category: "conditioning/prompt_relay".to_string(),
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
                m.insert("latent".to_string(), InputTypeSpec {
                    type_name: "LATENT".to_string(),
                    extra: HashMap::new(),
                });
                m.insert("global_prompt".to_string(), InputTypeSpec {
                    type_name: "STRING".to_string(),
                    extra: {
                        let mut e = HashMap::new();
                        e.insert("multiline".to_string(), json!(true));
                        e.insert("placeholder".to_string(), json!("Leave empty to use first segment text"));
                        e
                    },
                });
                m.insert("smart_prompt".to_string(), InputTypeSpec {
                    type_name: "STRING".to_string(),
                    extra: {
                        let mut e = HashMap::new();
                        e.insert("multiline".to_string(), json!(true));
                        e
                    },
                });
                m.insert("normalize_by_tokens".to_string(), InputTypeSpec {
                    type_name: "BOOLEAN".to_string(),
                    extra: {
                        let mut e = HashMap::new();
                        e.insert("default".to_string(), json!(false));
                        e
                    },
                });
                m.insert("epsilon".to_string(), InputTypeSpec {
                    type_name: "FLOAT".to_string(),
                    extra: {
                        let mut e = HashMap::new();
                        e.insert("default".to_string(), json!(0.001));
                        e.insert("min".to_string(), json!(0.000001));
                        e.insert("max".to_string(), json!(0.99));
                        e.insert("step".to_string(), json!(0.0001));
                        e
                    },
                });
                m
            },
            optional: {
                let mut m = HashMap::new();
                m.insert("relay_options".to_string(), InputTypeSpec {
                    type_name: "RELAY_OPTIONS".to_string(),
                    extra: HashMap::new(),
                });
                m
            },
            hidden: HashMap::new(),
        },
        output_types: vec![IoType::Model, IoType::Conditioning],
        output_names: vec!["MODEL".to_string(), "CONDITIONING".to_string()],
        output_is_list: vec![false, false],
        is_output_node: false,
        has_intermediate_output: false,
        is_changed: None,
        not_idempotent: false,
        function_name: "encode".to_string(),
    };

    registry.register(class_def, Arc::new(|ctx, _node, node_id| {
        let model = ctx.resolve_input(node_id, "model")
            .unwrap_or_else(|_| json!({}));
        let clip = ctx.resolve_input(node_id, "clip")
            .unwrap_or_else(|_| json!(null));
        let latent = ctx.resolve_input(node_id, "latent")
            .unwrap_or_else(|_| json!({}));
        let global_prompt = ctx.resolve_input(node_id, "global_prompt")
            .unwrap_or_else(|_| json!(""))
            .as_str()
            .unwrap_or("")
            .to_string();
        let smart_prompt = ctx.resolve_input(node_id, "smart_prompt")
            .unwrap_or_else(|_| json!(""))
            .as_str()
            .unwrap_or("")
            .to_string();
        let normalize_by_tokens = ctx.resolve_input(node_id, "normalize_by_tokens")
            .unwrap_or_else(|_| json!(false))
            .as_bool()
            .unwrap_or(false);
        let epsilon = ctx.resolve_input(node_id, "epsilon")
            .unwrap_or_else(|_| json!(0.001))
            .as_f64()
            .unwrap_or(0.001) as f32;
        let relay_options = ctx.resolve_input(node_id, "relay_options")
            .ok();

        Box::pin(async move {
            let mut model_out = model.as_object().cloned().unwrap_or_default();

            // Parse smart prompt
            let mut segments = parser::parse_smart_prompt(&smart_prompt);

            // Filter empty segments
            segments.retain(|s| !s.text.trim().is_empty());

            // Default segment if empty
            if segments.is_empty() {
                segments.push(parser::Segment {
                    text: "a video".to_string(),
                    weight: 1.0,
                });
            }

            // If normalize_by_tokens is enabled, multiply weight by token count
            // (simplified: use character count as proxy since we don't have tokenizer access)
            if normalize_by_tokens {
                for seg in &mut segments {
                    // Approximate token count: words count
                    let token_count = seg.text.split_whitespace().count().max(1) as f64;
                    seg.weight *= token_count;
                }
            }

            // Build local_prompts and segment_lengths from parsed segments
            let local_prompts: String = segments.iter()
                .map(|s| s.text.as_str())
                .collect::<Vec<&str>>()
                .join(" | ");

            let segment_lengths: String = segments.iter()
                .map(|s| {
                    // Scale weight to integer frame count (multiply by large factor)
                    let frame_count = (s.weight * 100000.0).round() as i64;
                    frame_count.to_string()
                })
                .collect::<Vec<String>>()
                .join(",");

            // Auto-fill global_prompt from first segment if empty
            let global_prompt = if global_prompt.is_empty() {
                segments[0].text.clone()
            } else {
                global_prompt
            };

            let mut relay_config = json!({
                "global_prompt": global_prompt,
                "local_prompts": local_prompts,
                "segment_lengths": segment_lengths,
                "epsilon": epsilon,
                "smart_prompt": smart_prompt,
                "normalize_by_tokens": normalize_by_tokens,
            });

            if let Some(opts) = relay_options {
                if let Some(opts_obj) = opts.as_object() {
                    if let Some(config_obj) = relay_config.as_object_mut() {
                        for (k, v) in opts_obj {
                            config_obj.insert(k.clone(), v.clone());
                        }
                    }
                }
            }

            if let Some(latent_obj) = latent.as_object() {
                if let Some(samples) = latent_obj.get("samples") {
                    if let Some(samples_obj) = samples.as_object() {
                        if let Some(frames) = samples_obj.get("length") {
                            if let Some(config_obj) = relay_config.as_object_mut() {
                                config_obj.insert("latent_frames".to_string(), frames.clone());
                            }
                        }
                    }
                }
            }

            model_out.insert("prompt_relay".to_string(), relay_config);

            let local_parts: Vec<&str> = local_prompts.split('|').map(|s| s.trim()).filter(|s| !s.is_empty()).collect();
            let combined_text = if local_parts.is_empty() {
                global_prompt.clone()
            } else {
                format!("{} {}", global_prompt, local_parts.join(" "))
            };

            let conditioning = json!({
                "type": "conditioning",
                "text": combined_text,
                "clip": clip,
                "prompt_relay": {
                    "global_prompt": global_prompt,
                    "local_prompts": local_prompts,
                    "segment_lengths": segment_lengths,
                    "epsilon": epsilon,
                },
            });

            Ok(vec![json!(model_out), conditioning])
        })
    }));
}

/// PromptRelaySmartEncodeTest - Smart syntax test/debug node.
///
/// Parses smart prompt and returns formatted output for debugging.
fn register_prompt_relay_smart_encode_test(registry: &mut NodeRegistry) {
    let class_def = NodeClassDef {
        class_type: "PromptRelaySmartEncodeTest".to_string(),
        display_name: "Prompt Relay Smart Encode Test".to_string(),
        category: "conditioning/prompt_relay".to_string(),
        input_types: NodeInputTypes {
            required: {
                let mut m = HashMap::new();
                m.insert("smart_prompt".to_string(), InputTypeSpec {
                    type_name: "STRING".to_string(),
                    extra: {
                        let mut e = HashMap::new();
                        e.insert("multiline".to_string(), json!(true));
                        e
                    },
                });
                m.insert("normalize_by_tokens".to_string(), InputTypeSpec {
                    type_name: "BOOLEAN".to_string(),
                    extra: {
                        let mut e = HashMap::new();
                        e.insert("default".to_string(), json!(false));
                        e
                    },
                });
                m
            },
            optional: {
                let mut m = HashMap::new();
                m.insert("clip".to_string(), InputTypeSpec {
                    type_name: "CLIP".to_string(),
                    extra: HashMap::new(),
                });
                m
            },
            hidden: HashMap::new(),
        },
        output_types: vec![IoType::String],
        output_names: vec!["parsed_output".to_string()],
        output_is_list: vec![false],
        is_output_node: false,
        has_intermediate_output: false,
        is_changed: None,
        not_idempotent: false,
        function_name: "test_parse".to_string(),
    };

    registry.register(class_def, Arc::new(|ctx, _node, node_id| {
        let smart_prompt = ctx.resolve_input(node_id, "smart_prompt")
            .unwrap_or_else(|_| json!(""))
            .as_str()
            .unwrap_or("")
            .to_string();
        let normalize_by_tokens = ctx.resolve_input(node_id, "normalize_by_tokens")
            .unwrap_or_else(|_| json!(false))
            .as_bool()
            .unwrap_or(false);

        Box::pin(async move {
            let mut segments = parser::parse_smart_prompt(&smart_prompt);
            segments.retain(|s| !s.text.trim().is_empty());

            if segments.is_empty() {
                return Ok(vec![json!("No segments parsed.")]);
            }

            if normalize_by_tokens {
                for seg in &mut segments {
                    let token_count = seg.text.split_whitespace().count().max(1) as f64;
                    seg.weight *= token_count;
                }
            }

            let local_prompts: String = segments.iter()
                .map(|s| s.text.as_str())
                .collect::<Vec<&str>>()
                .join(" | ");

            let segment_lengths: String = segments.iter()
                .map(|s| {
                    let frame_count = (s.weight * 100000.0).round() as i64;
                    frame_count.to_string()
                })
                .collect::<Vec<String>>()
                .join(",");

            let mut output = String::new();
            output.push_str(&format!("Parsed {} segment(s):\n\n", segments.len()));
            for (i, seg) in segments.iter().enumerate() {
                output.push_str(&format!("  [{}] text: {:?}\n", i + 1, seg.text));
                output.push_str(&format!("       weight: {:.2}\n\n", seg.weight));
            }
            output.push_str(&format!("local_prompts: {:?}\n", local_prompts));
            output.push_str(&format!("segment_lengths: {}\n", segment_lengths));
            output.push_str(&format!("global_prompt (auto): {:?}", segments[0].text));

            Ok(vec![json!(output)])
        })
    }));
}

/// PromptRelayAdvancedOptions - Advanced options node.
///
/// Provides fine-grained control over temporal penalty parameters
/// for video and audio streams.
fn register_prompt_relay_advanced_options(registry: &mut NodeRegistry) {
    let class_def = NodeClassDef {
        class_type: "PromptRelayAdvancedOptions".to_string(),
        display_name: "Prompt Relay Advanced Options".to_string(),
        category: "conditioning/prompt_relay".to_string(),
        input_types: NodeInputTypes {
            required: {
                let mut m = HashMap::new();
                m.insert("video_strength".to_string(), InputTypeSpec {
                    type_name: "FLOAT".to_string(),
                    extra: {
                        let mut e = HashMap::new();
                        e.insert("default".to_string(), json!(1.0));
                        e.insert("min".to_string(), json!(0.0));
                        e.insert("max".to_string(), json!(10.0));
                        e.insert("step".to_string(), json!(0.01));
                        e
                    },
                });
                m.insert("video_window_scale".to_string(), InputTypeSpec {
                    type_name: "FLOAT".to_string(),
                    extra: {
                        let mut e = HashMap::new();
                        e.insert("default".to_string(), json!(1.0));
                        e.insert("min".to_string(), json!(0.0));
                        e.insert("max".to_string(), json!(4.0));
                        e.insert("step".to_string(), json!(0.01));
                        e
                    },
                });
                m.insert("audio_epsilon".to_string(), InputTypeSpec {
                    type_name: "FLOAT".to_string(),
                    extra: {
                        let mut e = HashMap::new();
                        e.insert("default".to_string(), json!(0.0));
                        e.insert("min".to_string(), json!(0.0));
                        e.insert("max".to_string(), json!(0.99));
                        e.insert("step".to_string(), json!(0.001));
                        e
                    },
                });
                m.insert("audio_strength".to_string(), InputTypeSpec {
                    type_name: "FLOAT".to_string(),
                    extra: {
                        let mut e = HashMap::new();
                        e.insert("default".to_string(), json!(1.0));
                        e.insert("min".to_string(), json!(0.0));
                        e.insert("max".to_string(), json!(10.0));
                        e.insert("step".to_string(), json!(0.01));
                        e
                    },
                });
                m.insert("audio_window_scale".to_string(), InputTypeSpec {
                    type_name: "FLOAT".to_string(),
                    extra: {
                        let mut e = HashMap::new();
                        e.insert("default".to_string(), json!(1.0));
                        e.insert("min".to_string(), json!(0.0));
                        e.insert("max".to_string(), json!(4.0));
                        e.insert("step".to_string(), json!(0.01));
                        e
                    },
                });
                m
            },
            optional: HashMap::new(),
            hidden: HashMap::new(),
        },
        output_types: vec![IoType::RelayOptions],
        output_names: vec!["RELAY_OPTIONS".to_string()],
        output_is_list: vec![false],
        is_output_node: false,
        has_intermediate_output: false,
        is_changed: None,
        not_idempotent: false,
        function_name: "get_options".to_string(),
    };

    registry.register(class_def, Arc::new(|ctx, _node, node_id| {
        let video_strength = ctx.resolve_input(node_id, "video_strength")
            .unwrap_or_else(|_| json!(1.0))
            .as_f64()
            .unwrap_or(1.0) as f32;
        let video_window_scale = ctx.resolve_input(node_id, "video_window_scale")
            .unwrap_or_else(|_| json!(1.0))
            .as_f64()
            .unwrap_or(1.0) as f32;
        let audio_epsilon = ctx.resolve_input(node_id, "audio_epsilon")
            .unwrap_or_else(|_| json!(0.0))
            .as_f64()
            .unwrap_or(0.0) as f32;
        let audio_strength = ctx.resolve_input(node_id, "audio_strength")
            .unwrap_or_else(|_| json!(1.0))
            .as_f64()
            .unwrap_or(1.0) as f32;
        let audio_window_scale = ctx.resolve_input(node_id, "audio_window_scale")
            .unwrap_or_else(|_| json!(1.0))
            .as_f64()
            .unwrap_or(1.0) as f32;

        Box::pin(async move {
            Ok(vec![json!({
                "video_strength": video_strength,
                "video_window_scale": video_window_scale,
                "audio_epsilon": audio_epsilon,
                "audio_strength": audio_strength,
                "audio_window_scale": audio_window_scale,
            })])
        })
    }));
}

// ─── WhatDreamsCost Nodes ────────────────────────────────────────────────────

/// LTXKeyframer - Replaces video latent frames with encoded input images.
///
/// Takes batched images from MultiImageLoader and inserts them at specified
/// frame positions with adjustable strengths.
fn register_ltx_keyframer(registry: &mut NodeRegistry) {
    let class_def = NodeClassDef {
        class_type: "LTXKeyframer".to_string(),
        display_name: "LTX Keyframer".to_string(),
        category: "WhatDreamsCost".to_string(),
        input_types: NodeInputTypes {
            required: {
                let mut m = HashMap::new();
                m.insert("vae".to_string(), InputTypeSpec {
                    type_name: "VAE".to_string(),
                    extra: HashMap::new(),
                });
                m.insert("latent".to_string(), InputTypeSpec {
                    type_name: "LATENT".to_string(),
                    extra: HashMap::new(),
                });
                m.insert("multi_input".to_string(), InputTypeSpec {
                    type_name: "IMAGE".to_string(),
                    extra: HashMap::new(),
                });
                m.insert("num_images".to_string(), InputTypeSpec {
                    type_name: "INT".to_string(),
                    extra: {
                        let mut e = HashMap::new();
                        e.insert("default".to_string(), json!(1));
                        e.insert("min".to_string(), json!(0));
                        e.insert("max".to_string(), json!(50));
                        e.insert("step".to_string(), json!(1));
                        e
                    },
                });
                m
            },
            optional: {
                let mut m = HashMap::new();
                // Dynamic insert_frame_N and strength_N inputs (1..=50)
                for i in 1..=50 {
                    m.insert(format!("insert_frame_{}", i), InputTypeSpec {
                        type_name: "INT".to_string(),
                        extra: {
                            let mut e = HashMap::new();
                            e.insert("default".to_string(), json!(0));
                            e.insert("min".to_string(), json!(-9999));
                            e.insert("max".to_string(), json!(9999));
                            e.insert("step".to_string(), json!(1));
                            e
                        },
                    });
                    m.insert(format!("strength_{}", i), InputTypeSpec {
                        type_name: "FLOAT".to_string(),
                        extra: {
                            let mut e = HashMap::new();
                            e.insert("default".to_string(), json!(1.0));
                            e.insert("min".to_string(), json!(0.0));
                            e.insert("max".to_string(), json!(1.0));
                            e.insert("step".to_string(), json!(0.01));
                            e
                        },
                    });
                }
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
        not_idempotent: false,
        function_name: "keyframe".to_string(),
    };

    registry.register(class_def, Arc::new(|ctx, _node, node_id| {
        let vae = ctx.resolve_input(node_id, "vae")
            .unwrap_or_else(|_| json!({}));
        let latent = ctx.resolve_input(node_id, "latent")
            .unwrap_or_else(|_| json!({}));
        let multi_input = ctx.resolve_input(node_id, "multi_input")
            .unwrap_or_else(|_| json!(null));
        let num_images = ctx.resolve_input(node_id, "num_images")
            .unwrap_or_else(|_| json!(1))
            .as_i64()
            .unwrap_or(1) as i32;

        // Collect per-image frame/strength data
        let mut keyframes = Vec::new();
        for i in 1..=num_images.min(50) {
            let insert_frame = ctx.resolve_input(node_id, &format!("insert_frame_{}", i))
                .unwrap_or_else(|_| json!(0))
                .as_i64()
                .unwrap_or(0) as i32;
            let strength = ctx.resolve_input(node_id, &format!("strength_{}", i))
                .unwrap_or_else(|_| json!(1.0))
                .as_f64()
                .unwrap_or(1.0) as f32;
            keyframes.push(json!({
                "image_index": i - 1,
                "insert_frame": insert_frame,
                "strength": strength,
            }));
        }

        Box::pin(async move {
            Ok(vec![json!({
                "type": "ltx_keyframer",
                "vae": vae,
                "latent": latent,
                "multi_input": multi_input,
                "keyframes": keyframes,
            })])
        })
    }));
}

/// LTXSequencer - Add multiple guide images at specified frame indices or seconds.
///
/// Extends LTXVAddGuide functionality with multi-image support and frame/second modes.
fn register_ltx_sequencer(registry: &mut NodeRegistry) {
    let class_def = NodeClassDef {
        class_type: "LTXSequencer".to_string(),
        display_name: "LTX Sequencer".to_string(),
        category: "WhatDreamsCost".to_string(),
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
                m.insert("vae".to_string(), InputTypeSpec {
                    type_name: "VAE".to_string(),
                    extra: HashMap::new(),
                });
                m.insert("latent".to_string(), InputTypeSpec {
                    type_name: "LATENT".to_string(),
                    extra: HashMap::new(),
                });
                m.insert("multi_input".to_string(), InputTypeSpec {
                    type_name: "IMAGE".to_string(),
                    extra: HashMap::new(),
                });
                m.insert("num_images".to_string(), InputTypeSpec {
                    type_name: "INT".to_string(),
                    extra: {
                        let mut e = HashMap::new();
                        e.insert("default".to_string(), json!(1));
                        e.insert("min".to_string(), json!(0));
                        e.insert("max".to_string(), json!(50));
                        e.insert("step".to_string(), json!(1));
                        e
                    },
                });
                m.insert("insert_mode".to_string(), InputTypeSpec {
                    type_name: "COMBO".to_string(),
                    extra: {
                        let mut e = HashMap::new();
                        e.insert("choices".to_string(), json!(["frames", "seconds"]));
                        e.insert("default".to_string(), json!(0));
                        e
                    },
                });
                m.insert("frame_rate".to_string(), InputTypeSpec {
                    type_name: "INT".to_string(),
                    extra: {
                        let mut e = HashMap::new();
                        e.insert("default".to_string(), json!(24));
                        e.insert("min".to_string(), json!(1));
                        e.insert("max".to_string(), json!(120));
                        e.insert("step".to_string(), json!(1));
                        e
                    },
                });
                m
            },
            optional: {
                let mut m = HashMap::new();
                for i in 1..=50 {
                    m.insert(format!("insert_frame_{}", i), InputTypeSpec {
                        type_name: "INT".to_string(),
                        extra: {
                            let mut e = HashMap::new();
                            e.insert("default".to_string(), json!(0));
                            e.insert("min".to_string(), json!(-9999));
                            e.insert("max".to_string(), json!(9999));
                            e.insert("step".to_string(), json!(1));
                            e
                        },
                    });
                    m.insert(format!("insert_second_{}", i), InputTypeSpec {
                        type_name: "FLOAT".to_string(),
                        extra: {
                            let mut e = HashMap::new();
                            e.insert("default".to_string(), json!(0.0));
                            e.insert("min".to_string(), json!(0.0));
                            e.insert("max".to_string(), json!(9999.0));
                            e.insert("step".to_string(), json!(0.1));
                            e
                        },
                    });
                    m.insert(format!("strength_{}", i), InputTypeSpec {
                        type_name: "FLOAT".to_string(),
                        extra: {
                            let mut e = HashMap::new();
                            e.insert("default".to_string(), json!(1.0));
                            e.insert("min".to_string(), json!(0.0));
                            e.insert("max".to_string(), json!(1.0));
                            e.insert("step".to_string(), json!(0.01));
                            e
                        },
                    });
                }
                m
            },
            hidden: HashMap::new(),
        },
        output_types: vec![IoType::Conditioning, IoType::Conditioning, IoType::Latent],
        output_names: vec!["POSITIVE".to_string(), "NEGATIVE".to_string(), "LATENT".to_string()],
        output_is_list: vec![false, false, false],
        is_output_node: false,
        has_intermediate_output: false,
        is_changed: None,
        not_idempotent: false,
        function_name: "sequence".to_string(),
    };

    registry.register(class_def, Arc::new(|ctx, _node, node_id| {
        let positive = ctx.resolve_input(node_id, "positive")
            .unwrap_or_else(|_| json!({}));
        let negative = ctx.resolve_input(node_id, "negative")
            .unwrap_or_else(|_| json!({}));
        let vae = ctx.resolve_input(node_id, "vae")
            .unwrap_or_else(|_| json!({}));
        let latent = ctx.resolve_input(node_id, "latent")
            .unwrap_or_else(|_| json!({}));
        let multi_input = ctx.resolve_input(node_id, "multi_input")
            .unwrap_or_else(|_| json!(null));
        let num_images = ctx.resolve_input(node_id, "num_images")
            .unwrap_or_else(|_| json!(1))
            .as_i64()
            .unwrap_or(1) as i32;
        let insert_mode = ctx.resolve_input(node_id, "insert_mode")
            .unwrap_or_else(|_| json!("frames"))
            .as_str()
            .unwrap_or("frames")
            .to_string();
        let frame_rate = ctx.resolve_input(node_id, "frame_rate")
            .unwrap_or_else(|_| json!(24))
            .as_i64()
            .unwrap_or(24) as i32;

        let mut keyframes = Vec::new();
        for i in 1..=num_images.min(50) {
            let insert_frame = ctx.resolve_input(node_id, &format!("insert_frame_{}", i))
                .unwrap_or_else(|_| json!(0))
                .as_i64()
                .unwrap_or(0) as i32;
            let insert_second = ctx.resolve_input(node_id, &format!("insert_second_{}", i))
                .unwrap_or_else(|_| json!(0.0))
                .as_f64()
                .unwrap_or(0.0) as f32;
            let strength = ctx.resolve_input(node_id, &format!("strength_{}", i))
                .unwrap_or_else(|_| json!(1.0))
                .as_f64()
                .unwrap_or(1.0) as f32;
            keyframes.push(json!({
                "image_index": i - 1,
                "insert_frame": insert_frame,
                "insert_second": insert_second,
                "strength": strength,
            }));
        }

        Box::pin(async move {
            let mut pos = positive.as_object().cloned().unwrap_or_default();
            pos.insert("ltx_sequencer".to_string(), json!({
                "keyframes": keyframes,
                "insert_mode": insert_mode,
                "frame_rate": frame_rate,
                "vae": vae,
                "multi_input": multi_input,
            }));

            Ok(vec![
                json!(pos),
                negative,
                json!({
                    "type": "ltx_sequencer_latent",
                    "latent": latent,
                    "keyframes": keyframes,
                    "insert_mode": insert_mode,
                    "frame_rate": frame_rate,
                    "vae": vae,
                    "multi_input": multi_input,
                }),
            ])
        })
    }));
}

/// LTXDirector - WYSIWYG timeline variant with visual editor.
///
/// Same as Prompt Relay Encode but with timeline editor support, audio, and guide images.
fn register_ltx_director(registry: &mut NodeRegistry) {
    let class_def = NodeClassDef {
        class_type: "LTXDirector".to_string(),
        display_name: "LTX Director".to_string(),
        category: "WhatDreamsCost".to_string(),
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
                m.insert("global_prompt".to_string(), InputTypeSpec {
                    type_name: "STRING".to_string(),
                    extra: {
                        let mut e = HashMap::new();
                        e.insert("multiline".to_string(), json!(true));
                        e.insert("default".to_string(), json!(""));
                        e
                    },
                });
                m.insert("duration_frames".to_string(), InputTypeSpec {
                    type_name: "INT".to_string(),
                    extra: {
                        let mut e = HashMap::new();
                        e.insert("default".to_string(), json!(120));
                        e.insert("min".to_string(), json!(1));
                        e.insert("max".to_string(), json!(10000));
                        e.insert("step".to_string(), json!(1));
                        e
                    },
                });
                m.insert("duration_seconds".to_string(), InputTypeSpec {
                    type_name: "FLOAT".to_string(),
                    extra: {
                        let mut e = HashMap::new();
                        e.insert("default".to_string(), json!(5.0));
                        e.insert("min".to_string(), json!(0.1));
                        e.insert("max".to_string(), json!(1000.0));
                        e.insert("step".to_string(), json!(0.01));
                        e
                    },
                });
                m.insert("timeline_data".to_string(), InputTypeSpec {
                    type_name: "STRING".to_string(),
                    extra: {
                        let mut e = HashMap::new();
                        e.insert("default".to_string(), json!(""));
                        e.insert("timelineHidden".to_string(), json!(true));
                        e
                    },
                });
                m.insert("local_prompts".to_string(), InputTypeSpec {
                    type_name: "STRING".to_string(),
                    extra: {
                        let mut e = HashMap::new();
                        e.insert("multiline".to_string(), json!(true));
                        e.insert("default".to_string(), json!(""));
                        e.insert("timelineHidden".to_string(), json!(true));
                        e
                    },
                });
                m.insert("segment_lengths".to_string(), InputTypeSpec {
                    type_name: "STRING".to_string(),
                    extra: {
                        let mut e = HashMap::new();
                        e.insert("default".to_string(), json!(""));
                        e.insert("timelineHidden".to_string(), json!(true));
                        e
                    },
                });
                m.insert("epsilon".to_string(), InputTypeSpec {
                    type_name: "FLOAT".to_string(),
                    extra: {
                        let mut e = HashMap::new();
                        e.insert("default".to_string(), json!(0.001));
                        e.insert("min".to_string(), json!(0.0001));
                        e.insert("max".to_string(), json!(0.99));
                        e.insert("step".to_string(), json!(0.0001));
                        e
                    },
                });
                m.insert("frame_rate".to_string(), InputTypeSpec {
                    type_name: "FLOAT".to_string(),
                    extra: {
                        let mut e = HashMap::new();
                        e.insert("default".to_string(), json!(24.0));
                        e.insert("min".to_string(), json!(1.0));
                        e.insert("max".to_string(), json!(240.0));
                        e.insert("step".to_string(), json!(1.0));
                        e
                    },
                });
                m.insert("display_mode".to_string(), InputTypeSpec {
                    type_name: "COMBO".to_string(),
                    extra: {
                        let mut e = HashMap::new();
                        e.insert("choices".to_string(), json!(["frames", "seconds"]));
                        e.insert("default".to_string(), json!(1));
                        e
                    },
                });
                m.insert("guide_strength".to_string(), InputTypeSpec {
                    type_name: "STRING".to_string(),
                    extra: {
                        let mut e = HashMap::new();
                        e.insert("default".to_string(), json!(""));
                        e.insert("timelineHidden".to_string(), json!(true));
                        e
                    },
                });
                m
            },
            optional: {
                let mut m = HashMap::new();
                m.insert("audio_vae".to_string(), InputTypeSpec {
                    type_name: "VAE".to_string(),
                    extra: HashMap::new(),
                });
                m.insert("optional_latent".to_string(), InputTypeSpec {
                    type_name: "LATENT".to_string(),
                    extra: HashMap::new(),
                });
                m.insert("use_custom_audio".to_string(), InputTypeSpec {
                    type_name: "BOOLEAN".to_string(),
                    extra: {
                        let mut e = HashMap::new();
                        e.insert("default".to_string(), json!(false));
                        e
                    },
                });
                m.insert("custom_width".to_string(), InputTypeSpec {
                    type_name: "INT".to_string(),
                    extra: {
                        let mut e = HashMap::new();
                        e.insert("default".to_string(), json!(0));
                        e.insert("min".to_string(), json!(0));
                        e.insert("max".to_string(), json!(8192));
                        e.insert("step".to_string(), json!(1));
                        e
                    },
                });
                m.insert("custom_height".to_string(), InputTypeSpec {
                    type_name: "INT".to_string(),
                    extra: {
                        let mut e = HashMap::new();
                        e.insert("default".to_string(), json!(0));
                        e.insert("min".to_string(), json!(0));
                        e.insert("max".to_string(), json!(8192));
                        e.insert("step".to_string(), json!(1));
                        e
                    },
                });
                m.insert("resize_method".to_string(), InputTypeSpec {
                    type_name: "COMBO".to_string(),
                    extra: {
                        let mut e = HashMap::new();
                        e.insert("choices".to_string(), json!(["maintain aspect ratio", "stretch to fit", "pad", "crop"]));
                        e.insert("default".to_string(), json!(0));
                        e
                    },
                });
                m.insert("divisible_by".to_string(), InputTypeSpec {
                    type_name: "INT".to_string(),
                    extra: {
                        let mut e = HashMap::new();
                        e.insert("default".to_string(), json!(32));
                        e.insert("min".to_string(), json!(1));
                        e.insert("max".to_string(), json!(256));
                        e.insert("step".to_string(), json!(1));
                        e
                    },
                });
                m.insert("img_compression".to_string(), InputTypeSpec {
                    type_name: "INT".to_string(),
                    extra: {
                        let mut e = HashMap::new();
                        e.insert("default".to_string(), json!(18));
                        e.insert("min".to_string(), json!(0));
                        e.insert("max".to_string(), json!(100));
                        e.insert("step".to_string(), json!(1));
                        e
                    },
                });
                m
            },
            hidden: HashMap::new(),
        },
        output_types: vec![
            IoType::Model,
            IoType::Conditioning,
            IoType::Latent,
            IoType::Latent,
            IoType::GuideData,
            IoType::Float,
            IoType::Audio,
        ],
        output_names: vec![
            "MODEL".to_string(),
            "POSITIVE".to_string(),
            "VIDEO_LATENT".to_string(),
            "AUDIO_LATENT".to_string(),
            "GUIDE_DATA".to_string(),
            "FRAME_RATE".to_string(),
            "COMBINED_AUDIO".to_string(),
        ],
        output_is_list: vec![false, false, false, false, false, false, false],
        is_output_node: false,
        has_intermediate_output: false,
        is_changed: None,
        not_idempotent: false,
        function_name: "direct".to_string(),
    };

    registry.register(class_def, Arc::new(|ctx, _node, node_id| {
        let model = ctx.resolve_input(node_id, "model")
            .unwrap_or_else(|_| json!({}));
        let clip = ctx.resolve_input(node_id, "clip")
            .unwrap_or_else(|_| json!(null));
        let global_prompt = ctx.resolve_input(node_id, "global_prompt")
            .unwrap_or_else(|_| json!(""))
            .as_str()
            .unwrap_or("")
            .to_string();
        let duration_frames = ctx.resolve_input(node_id, "duration_frames")
            .unwrap_or_else(|_| json!(120))
            .as_i64()
            .unwrap_or(120) as i32;
        let duration_seconds = ctx.resolve_input(node_id, "duration_seconds")
            .unwrap_or_else(|_| json!(5.0))
            .as_f64()
            .unwrap_or(5.0) as f32;
        let timeline_data = ctx.resolve_input(node_id, "timeline_data")
            .unwrap_or_else(|_| json!(""))
            .as_str()
            .unwrap_or("")
            .to_string();
        let local_prompts = ctx.resolve_input(node_id, "local_prompts")
            .unwrap_or_else(|_| json!(""))
            .as_str()
            .unwrap_or("")
            .to_string();
        let segment_lengths = ctx.resolve_input(node_id, "segment_lengths")
            .unwrap_or_else(|_| json!(""))
            .as_str()
            .unwrap_or("")
            .to_string();
        let guide_strength = ctx.resolve_input(node_id, "guide_strength")
            .unwrap_or_else(|_| json!(""))
            .as_str()
            .unwrap_or("")
            .to_string();
        let epsilon = ctx.resolve_input(node_id, "epsilon")
            .unwrap_or_else(|_| json!(0.001))
            .as_f64()
            .unwrap_or(0.001) as f32;
        let frame_rate = ctx.resolve_input(node_id, "frame_rate")
            .unwrap_or_else(|_| json!(24.0))
            .as_f64()
            .unwrap_or(24.0) as f32;
        let display_mode = ctx.resolve_input(node_id, "display_mode")
            .unwrap_or_else(|_| json!("seconds"))
            .as_str()
            .unwrap_or("seconds")
            .to_string();
        let audio_vae = ctx.resolve_input(node_id, "audio_vae")
            .ok();
        let optional_latent = ctx.resolve_input(node_id, "optional_latent")
            .ok();
        let use_custom_audio = ctx.resolve_input(node_id, "use_custom_audio")
            .unwrap_or_else(|_| json!(false))
            .as_bool()
            .unwrap_or(false);
        let custom_width = ctx.resolve_input(node_id, "custom_width")
            .unwrap_or_else(|_| json!(0))
            .as_i64()
            .unwrap_or(0) as i32;
        let custom_height = ctx.resolve_input(node_id, "custom_height")
            .unwrap_or_else(|_| json!(0))
            .as_i64()
            .unwrap_or(0) as i32;
        let resize_method = ctx.resolve_input(node_id, "resize_method")
            .unwrap_or_else(|_| json!("maintain aspect ratio"))
            .as_str()
            .unwrap_or("maintain aspect ratio")
            .to_string();
        let divisible_by = ctx.resolve_input(node_id, "divisible_by")
            .unwrap_or_else(|_| json!(32))
            .as_i64()
            .unwrap_or(32) as i32;
        let img_compression = ctx.resolve_input(node_id, "img_compression")
            .unwrap_or_else(|_| json!(18))
            .as_i64()
            .unwrap_or(18) as i32;

        Box::pin(async move {
            let mut model_out = model.as_object().cloned().unwrap_or_default();

            // Build prompt relay config
            let relay_config = json!({
                "global_prompt": global_prompt,
                "local_prompts": local_prompts,
                "segment_lengths": segment_lengths,
                "epsilon": epsilon,
                "duration_frames": duration_frames,
                "duration_seconds": duration_seconds,
                "timeline_data": timeline_data,
                "frame_rate": frame_rate,
                "display_mode": display_mode,
                "guide_strength": guide_strength,
            });
            model_out.insert("prompt_relay".to_string(), relay_config);

            // Build conditioning
            let local_parts: Vec<&str> = local_prompts.split('|').map(|s| s.trim()).filter(|s| !s.is_empty()).collect();
            let combined_text = if local_parts.is_empty() {
                global_prompt.clone()
            } else {
                format!("{} {}", global_prompt, local_parts.join(" "))
            };

            let conditioning = json!({
                "type": "conditioning",
                "text": combined_text,
                "clip": clip,
                "prompt_relay": {
                    "global_prompt": global_prompt,
                    "local_prompts": local_prompts,
                    "segment_lengths": segment_lengths,
                    "epsilon": epsilon,
                },
            });

            // Video latent (auto-generated or from optional input)
            let video_latent = optional_latent.unwrap_or_else(|| json!({
                "type": "empty_latent_video",
                "duration_frames": duration_frames,
            }));

            // Audio latent
            let audio_latent = json!({
                "type": "ltx_audio_latent",
                "use_custom_audio": use_custom_audio,
                "audio_vae": audio_vae,
            });

            // Guide data for LTXDirectorGuide
            let guide_data = json!({
                "type": "guide_data",
                "timeline_data": timeline_data,
                "guide_strength": guide_strength,
                "custom_width": custom_width,
                "custom_height": custom_height,
                "resize_method": resize_method,
                "divisible_by": divisible_by,
                "img_compression": img_compression,
                "duration_frames": duration_frames,
                "frame_rate": frame_rate,
            });

            // Combined audio
            let combined_audio = json!({
                "type": "combined_audio",
                "timeline_data": timeline_data,
                "duration_frames": duration_frames,
                "frame_rate": frame_rate,
            });

            Ok(vec![
                json!(model_out),
                conditioning,
                video_latent,
                audio_latent,
                guide_data,
                json!(frame_rate),
                combined_audio,
            ])
        })
    }));
}

/// LTXDirectorGuide - Applies guide images from LTXDirector at specified frame positions.
///
/// Takes guide_data from LTXDirector and encodes/applies guide images to conditioning.
fn register_ltx_director_guide(registry: &mut NodeRegistry) {
    let class_def = NodeClassDef {
        class_type: "LTXDirectorGuide".to_string(),
        display_name: "LTX Director Guide".to_string(),
        category: "WhatDreamsCost".to_string(),
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
                m.insert("vae".to_string(), InputTypeSpec {
                    type_name: "VAE".to_string(),
                    extra: HashMap::new(),
                });
                m.insert("latent".to_string(), InputTypeSpec {
                    type_name: "LATENT".to_string(),
                    extra: HashMap::new(),
                });
                m.insert("guide_data".to_string(), InputTypeSpec {
                    type_name: "GUIDE_DATA".to_string(),
                    extra: HashMap::new(),
                });
                m.insert("scale_by".to_string(), InputTypeSpec {
                    type_name: "FLOAT".to_string(),
                    extra: {
                        let mut e = HashMap::new();
                        e.insert("default".to_string(), json!(1.0));
                        e.insert("min".to_string(), json!(0.01));
                        e.insert("max".to_string(), json!(8.0));
                        e.insert("step".to_string(), json!(0.01));
                        e
                    },
                });
                m.insert("upscale_method".to_string(), InputTypeSpec {
                    type_name: "COMBO".to_string(),
                    extra: {
                        let mut e = HashMap::new();
                        e.insert("choices".to_string(), json!(["nearest-exact", "bilinear", "area", "bicubic", "bislerp"]));
                        e.insert("default".to_string(), json!(3));
                        e
                    },
                });
                m
            },
            optional: HashMap::new(),
            hidden: HashMap::new(),
        },
        output_types: vec![IoType::Conditioning, IoType::Conditioning, IoType::Latent],
        output_names: vec!["POSITIVE".to_string(), "NEGATIVE".to_string(), "LATENT".to_string()],
        output_is_list: vec![false, false, false],
        is_output_node: false,
        has_intermediate_output: false,
        is_changed: None,
        not_idempotent: false,
        function_name: "guide".to_string(),
    };

    registry.register(class_def, Arc::new(|ctx, _node, node_id| {
        let positive = ctx.resolve_input(node_id, "positive")
            .unwrap_or_else(|_| json!({}));
        let negative = ctx.resolve_input(node_id, "negative")
            .unwrap_or_else(|_| json!({}));
        let vae = ctx.resolve_input(node_id, "vae")
            .unwrap_or_else(|_| json!({}));
        let latent = ctx.resolve_input(node_id, "latent")
            .unwrap_or_else(|_| json!({}));
        let guide_data = ctx.resolve_input(node_id, "guide_data")
            .unwrap_or_else(|_| json!({}));
        let scale_by = ctx.resolve_input(node_id, "scale_by")
            .unwrap_or_else(|_| json!(1.0))
            .as_f64()
            .unwrap_or(1.0) as f32;
        let upscale_method = ctx.resolve_input(node_id, "upscale_method")
            .unwrap_or_else(|_| json!("bicubic"))
            .as_str()
            .unwrap_or("bicubic")
            .to_string();

        Box::pin(async move {
            let mut pos = positive.as_object().cloned().unwrap_or_default();
            pos.insert("ltx_director_guide".to_string(), json!({
                "guide_data": guide_data,
                "scale_by": scale_by,
                "upscale_method": upscale_method,
                "vae": vae,
            }));

            let mut lat = latent.as_object().cloned().unwrap_or_default();
            lat.insert("ltx_director_guide".to_string(), json!({
                "guide_data": guide_data,
                "scale_by": scale_by,
                "upscale_method": upscale_method,
                "vae": vae,
            }));

            Ok(vec![json!(pos), negative, json!(lat)])
        })
    }));
}

/// MultiImageLoader - Load and batch multiple images from file paths.
///
/// Supports resize, compression, and outputs both a batched tensor and individual images.
fn register_multi_image_loader(registry: &mut NodeRegistry) {
    let class_def = NodeClassDef {
        class_type: "MultiImageLoader".to_string(),
        display_name: "Multi Image Loader".to_string(),
        category: "WhatDreamsCost".to_string(),
        input_types: NodeInputTypes {
            required: {
                let mut m = HashMap::new();
                m.insert("image_paths".to_string(), InputTypeSpec {
                    type_name: "STRING".to_string(),
                    extra: {
                        let mut e = HashMap::new();
                        e.insert("multiline".to_string(), json!(true));
                        e.insert("default".to_string(), json!(""));
                        e
                    },
                });
                m.insert("width".to_string(), InputTypeSpec {
                    type_name: "INT".to_string(),
                    extra: {
                        let mut e = HashMap::new();
                        e.insert("default".to_string(), json!(0));
                        e.insert("min".to_string(), json!(0));
                        e.insert("max".to_string(), json!(8192));
                        e.insert("step".to_string(), json!(1));
                        e
                    },
                });
                m.insert("height".to_string(), InputTypeSpec {
                    type_name: "INT".to_string(),
                    extra: {
                        let mut e = HashMap::new();
                        e.insert("default".to_string(), json!(0));
                        e.insert("min".to_string(), json!(0));
                        e.insert("max".to_string(), json!(8192));
                        e.insert("step".to_string(), json!(1));
                        e
                    },
                });
                m.insert("interpolation".to_string(), InputTypeSpec {
                    type_name: "COMBO".to_string(),
                    extra: {
                        let mut e = HashMap::new();
                        e.insert("choices".to_string(), json!(["lanczos", "nearest", "bilinear", "bicubic", "area", "nearest-exact"]));
                        e.insert("default".to_string(), json!(0));
                        e
                    },
                });
                m.insert("resize_method".to_string(), InputTypeSpec {
                    type_name: "COMBO".to_string(),
                    extra: {
                        let mut e = HashMap::new();
                        e.insert("choices".to_string(), json!(["keep proportion", "stretch", "pad", "crop"]));
                        e.insert("default".to_string(), json!(0));
                        e
                    },
                });
                m.insert("multiple_of".to_string(), InputTypeSpec {
                    type_name: "INT".to_string(),
                    extra: {
                        let mut e = HashMap::new();
                        e.insert("default".to_string(), json!(32));
                        e.insert("min".to_string(), json!(0));
                        e.insert("max".to_string(), json!(512));
                        e.insert("step".to_string(), json!(1));
                        e
                    },
                });
                m.insert("img_compression".to_string(), InputTypeSpec {
                    type_name: "INT".to_string(),
                    extra: {
                        let mut e = HashMap::new();
                        e.insert("default".to_string(), json!(18));
                        e.insert("min".to_string(), json!(0));
                        e.insert("max".to_string(), json!(100));
                        e.insert("step".to_string(), json!(1));
                        e
                    },
                });
                m
            },
            optional: HashMap::new(),
            hidden: HashMap::new(),
        },
        output_types: vec![IoType::Image],
        output_names: vec!["MULTI_OUTPUT".to_string()],
        output_is_list: vec![false],
        is_output_node: false,
        has_intermediate_output: false,
        is_changed: None,
        not_idempotent: false,
        function_name: "load_images".to_string(),
    };

    registry.register(class_def, Arc::new(|ctx, _node, node_id| {
        let image_paths = ctx.resolve_input(node_id, "image_paths")
            .unwrap_or_else(|_| json!(""))
            .as_str()
            .unwrap_or("")
            .to_string();
        let width = ctx.resolve_input(node_id, "width")
            .unwrap_or_else(|_| json!(0))
            .as_i64()
            .unwrap_or(0) as i32;
        let height = ctx.resolve_input(node_id, "height")
            .unwrap_or_else(|_| json!(0))
            .as_i64()
            .unwrap_or(0) as i32;
        let interpolation = ctx.resolve_input(node_id, "interpolation")
            .unwrap_or_else(|_| json!("lanczos"))
            .as_str()
            .unwrap_or("lanczos")
            .to_string();
        let resize_method = ctx.resolve_input(node_id, "resize_method")
            .unwrap_or_else(|_| json!("keep proportion"))
            .as_str()
            .unwrap_or("keep proportion")
            .to_string();
        let multiple_of = ctx.resolve_input(node_id, "multiple_of")
            .unwrap_or_else(|_| json!(32))
            .as_i64()
            .unwrap_or(32) as i32;
        let img_compression = ctx.resolve_input(node_id, "img_compression")
            .unwrap_or_else(|_| json!(18))
            .as_i64()
            .unwrap_or(18) as i32;

        Box::pin(async move {
            let paths: Vec<&str> = image_paths.lines()
                .map(|s| s.trim())
                .filter(|s| !s.is_empty())
                .collect();

            Ok(vec![json!({
                "type": "multi_image_loader",
                "image_paths": paths,
                "width": width,
                "height": height,
                "interpolation": interpolation,
                "resize_method": resize_method,
                "multiple_of": multiple_of,
                "img_compression": img_compression,
                "count": paths.len(),
            })])
        })
    }));
}

/// SpeechLengthCalculator - Calculate speech duration from quoted text.
///
/// Parses text for quoted speech and estimates frame counts at different speaking rates.
fn register_speech_length_calculator(registry: &mut NodeRegistry) {
    let class_def = NodeClassDef {
        class_type: "SpeechLengthCalculator".to_string(),
        display_name: "Speech Length Calculator".to_string(),
        category: "WhatDreamsCost".to_string(),
        input_types: NodeInputTypes {
            required: {
                let mut m = HashMap::new();
                m.insert("text".to_string(), InputTypeSpec {
                    type_name: "STRING".to_string(),
                    extra: {
                        let mut e = HashMap::new();
                        e.insert("multiline".to_string(), json!(true));
                        e.insert("default".to_string(), json!("Enter your script here. \"Make sure to put spoken words inside quotes!\""));
                        e
                    },
                });
                m.insert("fps".to_string(), InputTypeSpec {
                    type_name: "INT".to_string(),
                    extra: {
                        let mut e = HashMap::new();
                        e.insert("default".to_string(), json!(24));
                        e.insert("min".to_string(), json!(1));
                        e.insert("max".to_string(), json!(120));
                        e.insert("step".to_string(), json!(1));
                        e
                    },
                });
                m.insert("additional_time".to_string(), InputTypeSpec {
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
            optional: {
                let mut m = HashMap::new();
                m.insert("text_input".to_string(), InputTypeSpec {
                    type_name: "STRING".to_string(),
                    extra: HashMap::new(),
                });
                m
            },
            hidden: HashMap::new(),
        },
        output_types: vec![IoType::Int, IoType::Int, IoType::Int, IoType::String],
        output_names: vec![
            "SLOW_FRAME_COUNT".to_string(),
            "AVERAGE_FRAME_COUNT".to_string(),
            "FAST_FRAME_COUNT".to_string(),
            "TEXT".to_string(),
        ],
        output_is_list: vec![false, false, false, false],
        is_output_node: false,
        has_intermediate_output: false,
        is_changed: None,
        not_idempotent: false,
        function_name: "calculate_speech".to_string(),
    };

    registry.register(class_def, Arc::new(|ctx, _node, node_id| {
        let text = ctx.resolve_input(node_id, "text")
            .unwrap_or_else(|_| json!(""))
            .as_str()
            .unwrap_or("")
            .to_string();
        let fps = ctx.resolve_input(node_id, "fps")
            .unwrap_or_else(|_| json!(24))
            .as_i64()
            .unwrap_or(24) as i32;
        let additional_time = ctx.resolve_input(node_id, "additional_time")
            .unwrap_or_else(|_| json!(0.0))
            .as_f64()
            .unwrap_or(0.0) as f32;
        let text_input = ctx.resolve_input(node_id, "text_input")
            .ok()
            .and_then(|v| v.as_str().map(|s| s.to_string()))
            .filter(|s| !s.trim().is_empty());

        Box::pin(async move {
            let active_text = text_input.unwrap_or_else(|| text.clone());

            // Extract quoted text (double, single, smart quotes)
            let re = regex::Regex::new(r#""([^"]*)"|'([^']*)'|"([^"]*)"|'([^']*)'"#).unwrap();
            let quoted_words: Vec<&str> = re.captures_iter(&active_text)
                .flat_map(|caps| {
                    caps.iter()
                        .skip(1)
                        .filter_map(|m| m.map(|m| m.as_str()))
                        .next()
                })
                .flat_map(|s| s.split_whitespace())
                .collect();
            let word_count = quoted_words.len() as f64;

            let calc_frames = |wpm: f64| -> i32 {
                if word_count == 0.0 && additional_time == 0.0 {
                    return 0;
                }
                let minutes = word_count / wpm;
                let seconds = minutes * 60.0 + additional_time as f64;
                (seconds * fps as f64).ceil() as i32
            };

            let slow_frames = calc_frames(100.0);
            let avg_frames = calc_frames(130.0);
            let fast_frames = calc_frames(160.0);

            Ok(vec![
                json!(slow_frames),
                json!(avg_frames),
                json!(fast_frames),
                json!(active_text),
            ])
        })
    }));
}

/// LoadAudioUI - Load audio with UI controls for trimming.
///
/// Provides an audio file picker with start/end/duration trimming controls.
fn register_load_audio_ui(registry: &mut NodeRegistry) {
    let class_def = NodeClassDef {
        class_type: "LoadAudioUI".to_string(),
        display_name: "Load Audio UI".to_string(),
        category: "WhatDreamsCost".to_string(),
        input_types: NodeInputTypes {
            required: {
                let mut m = HashMap::new();
                m.insert("audio".to_string(), InputTypeSpec {
                    type_name: "COMBO".to_string(),
                    extra: {
                        let mut e = HashMap::new();
                        e.insert("audio_upload".to_string(), json!(true));
                        e
                    },
                });
                m.insert("start_time".to_string(), InputTypeSpec {
                    type_name: "FLOAT".to_string(),
                    extra: {
                        let mut e = HashMap::new();
                        e.insert("default".to_string(), json!(0.0));
                        e.insert("min".to_string(), json!(0.0));
                        e.insert("max".to_string(), json!(100000.0));
                        e.insert("step".to_string(), json!(0.01));
                        e
                    },
                });
                m.insert("end_time".to_string(), InputTypeSpec {
                    type_name: "FLOAT".to_string(),
                    extra: {
                        let mut e = HashMap::new();
                        e.insert("default".to_string(), json!(0.0));
                        e.insert("min".to_string(), json!(0.0));
                        e.insert("max".to_string(), json!(100000.0));
                        e.insert("step".to_string(), json!(0.01));
                        e
                    },
                });
                m.insert("duration".to_string(), InputTypeSpec {
                    type_name: "FLOAT".to_string(),
                    extra: {
                        let mut e = HashMap::new();
                        e.insert("default".to_string(), json!(0.0));
                        e.insert("min".to_string(), json!(0.0));
                        e.insert("max".to_string(), json!(100000.0));
                        e.insert("step".to_string(), json!(0.01));
                        e
                    },
                });
                m
            },
            optional: HashMap::new(),
            hidden: HashMap::new(),
        },
        output_types: vec![IoType::Audio, IoType::Float],
        output_names: vec!["AUDIO".to_string(), "DURATION".to_string()],
        output_is_list: vec![false, false],
        is_output_node: false,
        has_intermediate_output: false,
        is_changed: None,
        not_idempotent: false,
        function_name: "load_audio".to_string(),
    };

    registry.register(class_def, Arc::new(|ctx, _node, node_id| {
        let audio = ctx.resolve_input(node_id, "audio")
            .unwrap_or_else(|_| json!("none"))
            .as_str()
            .unwrap_or("none")
            .to_string();
        let start_time = ctx.resolve_input(node_id, "start_time")
            .unwrap_or_else(|_| json!(0.0))
            .as_f64()
            .unwrap_or(0.0) as f32;
        let end_time = ctx.resolve_input(node_id, "end_time")
            .unwrap_or_else(|_| json!(0.0))
            .as_f64()
            .unwrap_or(0.0) as f32;

        Box::pin(async move {
            let audio_output = json!({
                "type": "load_audio",
                "audio_file": audio,
                "start_time": start_time,
                "end_time": end_time,
            });
            let duration = if end_time > 0.0 { end_time - start_time } else { 0.0 };

            Ok(vec![audio_output, json!(duration)])
        })
    }));
}

/// LoadVideoUI - Load video with UI controls for trimming, resizing, and cropping.
///
/// Provides video file loading with time/frame-based trimming, resize, and crop controls.
fn register_load_video_ui(registry: &mut NodeRegistry) {
    let class_def = NodeClassDef {
        class_type: "LoadVideoUI".to_string(),
        display_name: "Load Video UI".to_string(),
        category: "WhatDreamsCost".to_string(),
        input_types: NodeInputTypes {
            required: {
                let mut m = HashMap::new();
                m.insert("video".to_string(), InputTypeSpec {
                    type_name: "STRING".to_string(),
                    extra: {
                        let mut e = HashMap::new();
                        e.insert("default".to_string(), json!(""));
                        e
                    },
                });
                m.insert("start_time".to_string(), InputTypeSpec {
                    type_name: "FLOAT".to_string(),
                    extra: {
                        let mut e = HashMap::new();
                        e.insert("default".to_string(), json!(0.0));
                        e.insert("min".to_string(), json!(0.0));
                        e.insert("max".to_string(), json!(100000.0));
                        e.insert("step".to_string(), json!(0.01));
                        e
                    },
                });
                m.insert("end_time".to_string(), InputTypeSpec {
                    type_name: "FLOAT".to_string(),
                    extra: {
                        let mut e = HashMap::new();
                        e.insert("default".to_string(), json!(0.0));
                        e.insert("min".to_string(), json!(0.0));
                        e.insert("max".to_string(), json!(100000.0));
                        e.insert("step".to_string(), json!(0.01));
                        e
                    },
                });
                m.insert("duration".to_string(), InputTypeSpec {
                    type_name: "FLOAT".to_string(),
                    extra: {
                        let mut e = HashMap::new();
                        e.insert("default".to_string(), json!(0.0));
                        e.insert("min".to_string(), json!(0.0));
                        e.insert("max".to_string(), json!(100000.0));
                        e.insert("step".to_string(), json!(0.01));
                        e
                    },
                });
                m.insert("start_frame".to_string(), InputTypeSpec {
                    type_name: "INT".to_string(),
                    extra: {
                        let mut e = HashMap::new();
                        e.insert("default".to_string(), json!(0));
                        e.insert("min".to_string(), json!(0));
                        e.insert("max".to_string(), json!(10000000));
                        e.insert("step".to_string(), json!(1));
                        e
                    },
                });
                m.insert("end_frame".to_string(), InputTypeSpec {
                    type_name: "INT".to_string(),
                    extra: {
                        let mut e = HashMap::new();
                        e.insert("default".to_string(), json!(0));
                        e.insert("min".to_string(), json!(0));
                        e.insert("max".to_string(), json!(10000000));
                        e.insert("step".to_string(), json!(1));
                        e
                    },
                });
                m.insert("duration_frames".to_string(), InputTypeSpec {
                    type_name: "INT".to_string(),
                    extra: {
                        let mut e = HashMap::new();
                        e.insert("default".to_string(), json!(0));
                        e.insert("min".to_string(), json!(0));
                        e.insert("max".to_string(), json!(10000000));
                        e.insert("step".to_string(), json!(1));
                        e
                    },
                });
                m.insert("resize_method".to_string(), InputTypeSpec {
                    type_name: "COMBO".to_string(),
                    extra: {
                        let mut e = HashMap::new();
                        e.insert("choices".to_string(), json!(["maintain aspect ratio", "stretch to fit", "pad", "crop"]));
                        e.insert("default".to_string(), json!(0));
                        e
                    },
                });
                m.insert("custom_width".to_string(), InputTypeSpec {
                    type_name: "INT".to_string(),
                    extra: {
                        let mut e = HashMap::new();
                        e.insert("default".to_string(), json!(0));
                        e.insert("min".to_string(), json!(0));
                        e.insert("max".to_string(), json!(100000));
                        e.insert("step".to_string(), json!(8));
                        e
                    },
                });
                m.insert("custom_height".to_string(), InputTypeSpec {
                    type_name: "INT".to_string(),
                    extra: {
                        let mut e = HashMap::new();
                        e.insert("default".to_string(), json!(0));
                        e.insert("min".to_string(), json!(0));
                        e.insert("max".to_string(), json!(100000));
                        e.insert("step".to_string(), json!(8));
                        e
                    },
                });
                m.insert("frame_rate".to_string(), InputTypeSpec {
                    type_name: "INT".to_string(),
                    extra: {
                        let mut e = HashMap::new();
                        e.insert("default".to_string(), json!(24));
                        e.insert("min".to_string(), json!(1));
                        e.insert("max".to_string(), json!(120));
                        e.insert("step".to_string(), json!(1));
                        e
                    },
                });
                m.insert("display_mode".to_string(), InputTypeSpec {
                    type_name: "COMBO".to_string(),
                    extra: {
                        let mut e = HashMap::new();
                        e.insert("choices".to_string(), json!(["seconds", "frames"]));
                        e.insert("default".to_string(), json!(0));
                        e
                    },
                });
                m.insert("crop_x".to_string(), InputTypeSpec {
                    type_name: "FLOAT".to_string(),
                    extra: {
                        let mut e = HashMap::new();
                        e.insert("default".to_string(), json!(0.0));
                        e.insert("min".to_string(), json!(0.0));
                        e.insert("max".to_string(), json!(1.0));
                        e.insert("step".to_string(), json!(0.001));
                        e
                    },
                });
                m.insert("crop_y".to_string(), InputTypeSpec {
                    type_name: "FLOAT".to_string(),
                    extra: {
                        let mut e = HashMap::new();
                        e.insert("default".to_string(), json!(0.0));
                        e.insert("min".to_string(), json!(0.0));
                        e.insert("max".to_string(), json!(1.0));
                        e.insert("step".to_string(), json!(0.001));
                        e
                    },
                });
                m.insert("crop_w".to_string(), InputTypeSpec {
                    type_name: "FLOAT".to_string(),
                    extra: {
                        let mut e = HashMap::new();
                        e.insert("default".to_string(), json!(1.0));
                        e.insert("min".to_string(), json!(0.0));
                        e.insert("max".to_string(), json!(1.0));
                        e.insert("step".to_string(), json!(0.001));
                        e
                    },
                });
                m.insert("crop_h".to_string(), InputTypeSpec {
                    type_name: "FLOAT".to_string(),
                    extra: {
                        let mut e = HashMap::new();
                        e.insert("default".to_string(), json!(1.0));
                        e.insert("min".to_string(), json!(0.0));
                        e.insert("max".to_string(), json!(1.0));
                        e.insert("step".to_string(), json!(0.001));
                        e
                    },
                });
                m
            },
            optional: HashMap::new(),
            hidden: HashMap::new(),
        },
        output_types: vec![IoType::Image, IoType::Audio, IoType::Float, IoType::Int],
        output_names: vec![
            "IMAGES".to_string(),
            "AUDIO".to_string(),
            "DURATION".to_string(),
            "FRAME_COUNT".to_string(),
        ],
        output_is_list: vec![false, false, false, false],
        is_output_node: false,
        has_intermediate_output: false,
        is_changed: None,
        not_idempotent: false,
        function_name: "load_video".to_string(),
    };

    registry.register(class_def, Arc::new(|ctx, _node, node_id| {
        let video = ctx.resolve_input(node_id, "video")
            .unwrap_or_else(|_| json!(""))
            .as_str()
            .unwrap_or("")
            .to_string();
        let start_time = ctx.resolve_input(node_id, "start_time")
            .unwrap_or_else(|_| json!(0.0))
            .as_f64()
            .unwrap_or(0.0) as f32;
        let end_time = ctx.resolve_input(node_id, "end_time")
            .unwrap_or_else(|_| json!(0.0))
            .as_f64()
            .unwrap_or(0.0) as f32;
        let start_frame = ctx.resolve_input(node_id, "start_frame")
            .unwrap_or_else(|_| json!(0))
            .as_i64()
            .unwrap_or(0) as i32;
        let end_frame = ctx.resolve_input(node_id, "end_frame")
            .unwrap_or_else(|_| json!(0))
            .as_i64()
            .unwrap_or(0) as i32;
        let frame_rate = ctx.resolve_input(node_id, "frame_rate")
            .unwrap_or_else(|_| json!(24))
            .as_i64()
            .unwrap_or(24) as i32;
        let display_mode = ctx.resolve_input(node_id, "display_mode")
            .unwrap_or_else(|_| json!("seconds"))
            .as_str()
            .unwrap_or("seconds")
            .to_string();
        let resize_method = ctx.resolve_input(node_id, "resize_method")
            .unwrap_or_else(|_| json!("maintain aspect ratio"))
            .as_str()
            .unwrap_or("maintain aspect ratio")
            .to_string();
        let custom_width = ctx.resolve_input(node_id, "custom_width")
            .unwrap_or_else(|_| json!(0))
            .as_i64()
            .unwrap_or(0) as i32;
        let custom_height = ctx.resolve_input(node_id, "custom_height")
            .unwrap_or_else(|_| json!(0))
            .as_i64()
            .unwrap_or(0) as i32;
        let crop_x = ctx.resolve_input(node_id, "crop_x")
            .unwrap_or_else(|_| json!(0.0))
            .as_f64()
            .unwrap_or(0.0) as f32;
        let crop_y = ctx.resolve_input(node_id, "crop_y")
            .unwrap_or_else(|_| json!(0.0))
            .as_f64()
            .unwrap_or(0.0) as f32;
        let crop_w = ctx.resolve_input(node_id, "crop_w")
            .unwrap_or_else(|_| json!(1.0))
            .as_f64()
            .unwrap_or(1.0) as f32;
        let crop_h = ctx.resolve_input(node_id, "crop_h")
            .unwrap_or_else(|_| json!(1.0))
            .as_f64()
            .unwrap_or(1.0) as f32;

        Box::pin(async move {
            let images = json!({
                "type": "load_video",
                "video_file": video,
                "start_time": start_time,
                "end_time": end_time,
                "start_frame": start_frame,
                "end_frame": end_frame,
                "frame_rate": frame_rate,
                "display_mode": display_mode,
                "resize_method": resize_method,
                "custom_width": custom_width,
                "custom_height": custom_height,
                "crop_x": crop_x,
                "crop_y": crop_y,
                "crop_w": crop_w,
                "crop_h": crop_h,
            });
            let audio = json!({
                "type": "video_audio",
                "video_file": video,
                "start_time": start_time,
                "end_time": end_time,
            });
            let duration = if end_time > 0.0 { end_time - start_time } else { 0.0 };
            let frame_count = if end_frame > 0 { end_frame - start_frame } else { 0 };

            Ok(vec![images, audio, json!(duration), json!(frame_count)])
        })
    }));
}

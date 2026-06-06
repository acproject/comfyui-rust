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

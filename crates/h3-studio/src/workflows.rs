//! Parameterized workflow graphs for the 4 flows defined in H3_Studio_Design
//! and the four JSON files (去背景 / 参卡图生成电影画质图 / 将人物切换为4视图 /
//! h3_refmod_bamboo_s_workflow2).
//!
//! Each graph is a plain JSON object ready for POST /prompt.

use serde_json::{json, Value};

/// Remove background — 去背景.json
pub fn remove_background_workflow(image: &str, prefix: &str, model: &str) -> Value {
    json!({
        "1": { "class_type": "LoadImage", "inputs": { "image": image } },
        "2": { "class_type": "RemoveBackground", "inputs": { "image": ["1", 0], "model": model } },
        "3": { "class_type": "SaveImage", "inputs": { "filename_prefix": prefix, "images": ["2", 0] } }
    })
}

/// Cinematic still from reference — 参卡图生成电影画质图.json
pub struct CinematicParams {
    pub image: String,
    pub prompt: String,
    pub negative_prompt: String,
    pub model: String,
    pub cfg: f32,
    pub steps: u32,
    pub width: u32,
    pub height: u32,
    pub seed: u32,
    pub prefix: String,
}

pub const DEFAULT_CINE_PROMPT: &str = "Cinematic live-action film still, 16:9 widescreen, a real young woman standing in a dense towering bamboo forest, bright midday, the sun is high overhead but 80% of the sunlight is blocked by the thick bamboo leaf canopy, only soft dappled light patches and faint volumetric god rays filter through small gaps, her clothing, outfit color, fabric texture, and overall styling must exactly match the reference image, natural skin texture with visible pores and subtle imperfections, hair gently swaying in a light breeze, looking slightly off-camera with a calm contemplative expression, narrow mossy path between tall slender bamboo stalks, slight ground-level mist, shallow depth of field with creamy bokeh, shot on ARRI Alexa 65, 35mm anamorphic lens, T2.0, natural color grading with muted teal-green tones, subtle organic film grain, photorealistic, hyper-detailed, 8K, rule of thirds composition, atmospheric depth, live action, real human, documentary cinematography";

pub const DEFAULT_CINE_NEGATIVE: &str = "anime, manga, cartoon, illustration, painting, sketch, watercolor, 3d render, cgi, digital art, unreal engine, blender, pixar, disney style, stylized, exaggerated features, plastic skin, airbrushed, over-smoothed, cartoonish eyes, chibi, afternoon, sunset, sunrise, golden hour, orange light, harsh direct sunlight, overexposed, low quality, blurry, deformed, extra fingers, bad anatomy, text, watermark, logo, oversaturated, flat lighting, night, dark, snow, rain, people in background, modern buildings, phone, car";

pub fn cinematic_workflow(p: &CinematicParams) -> Value {
    json!({
        "1": { "class_type": "LoadImage", "inputs": { "image": p.image } },
        "2": {
            "class_type": "QwenImagePipeline",
            "inputs": {
                "prompt": p.prompt,
                "qwen_model": p.model,
                "cfg": p.cfg,
                "height": p.height,
                "image": ["1", 0],
                "negative_prompt": p.negative_prompt,
                "seed": p.seed,
                "steps": p.steps,
                "width": p.width
            }
        },
        "3": { "class_type": "SaveImage", "inputs": { "filename_prefix": p.prefix, "images": ["2", 0] } }
    })
}

/// Multi-view turnaround — 将人物切换为4视图.json
pub struct MultiViewParams {
    pub image: String,
    pub instructions: String,      // 1 line per view, e.g. "front view\nside view\nback view\ntop-down view"
    pub subject_guidance: String,  // identity/style lock
    pub lora: String,
    pub lora_strength: f32,
    pub negative_prompt: String,
    pub model: String,
    pub cfg: f32,
    pub steps: u32,
    pub width: u32,
    pub height: u32,
    pub seed: u32,
    pub prefix: String,
    pub save_as: SaveAs,           // merged grid or per-view
}

#[derive(Clone, Copy, PartialEq)]
pub enum SaveAs { Merged, PerView }

impl SaveAs {
    pub fn as_str(&self) -> &'static str {
        match self {
            SaveAs::Merged => "merged",
            SaveAs::PerView => "per-view",
        }
    }
}

pub const DEFAULT_MULTI_INSTRUCTIONS: &str = "front view\nside view\nback view\ntop-down view";
pub const DEFAULT_MULTI_GUIDANCE: &str = "keep the same face, same hairstyle, same clothing";
pub const DEFAULT_LORA: &str = "qwen-edit-2509-multiple-angles.safetensors";

pub fn multiview_workflow(p: &MultiViewParams) -> Value {
    if p.save_as == SaveAs::PerView {
        // Per-view: SaveImage directly from the multi-view node → N files.
        return json!({
            "1": { "class_type": "LoadImage", "inputs": { "image": p.image } },
            "2": {
                "class_type": "QwenImageMultiViewPose",
                "inputs": {
                    "image": ["1", 0],
                    "instructions": p.instructions,
                    "qwen_model": p.model,
                    "cfg": p.cfg,
                    "height": p.height,
                    "lora": p.lora,
                    "lora_strength": p.lora_strength,
                    "negative_prompt": p.negative_prompt,
                    "seed": p.seed,
                    "steps": p.steps,
                    "subject_guidance": p.subject_guidance,
                    "width": p.width
                }
            },
            "3": { "class_type": "SaveImage", "inputs": { "filename_prefix": p.prefix, "images": ["2", 0] } }
        });
    }
    // Merged 2×2 grid.
    json!({
        "1": { "class_type": "LoadImage", "inputs": { "image": p.image } },
        "2": {
            "class_type": "QwenImageMultiViewPose",
            "inputs": {
                "image": ["1", 0],
                "instructions": p.instructions,
                "qwen_model": p.model,
                "cfg": p.cfg,
                "height": p.height,
                "lora": p.lora,
                "lora_strength": p.lora_strength,
                "negative_prompt": p.negative_prompt,
                "seed": p.seed,
                "steps": p.steps,
                "subject_guidance": p.subject_guidance,
                "width": p.width
            }
        },
        "3": { "class_type": "ImageGrid", "inputs": { "images": ["2", 0], "columns": 2 } },
        "4": { "class_type": "SaveImage", "inputs": { "filename_prefix": p.prefix, "images": ["3", 0] } }
    })
}

/// SaveVideo inputs — format default "mp4".
pub fn save_video(video_node: &str, prefix: &str, fps: u32) -> Value {
    json!({ "class_type": "SaveVideo", "inputs": { "video": [video_node, 0], "filename_prefix": prefix, "format": "mp4", "fps": fps } })
}

/// Video Ref2VA — h3_refmod_bamboo_s_workflow2.json
pub struct Ref2VaParams {
    pub image: String,
    pub view_descriptions: [String; 4],
    pub vae_dir: String,
    pub prompt: String,
    pub embed_frames: bool,
    pub fps: u32,
    pub height: u32,
    pub width: u32,
    pub num_frames: u32,
    pub seed: u32,
    pub split_gpu: bool,
    pub steps: u32,
    pub strength: f32,
    pub prefix: String,
}

pub fn ref2va_workflow(p: &Ref2VaParams) -> Value {
    let names = ["bw_view_front", "bw_view_side", "bw_view_back", "bw_view_top"];
    // LoadImage ×4 → RefModExtract ×4
    let mut graph = serde_json::Map::new();
    let extract_ids: Vec<String> = vec!["extract_1", "extract_2", "extract_3", "extract_4"].iter().map(|s| s.to_string()).collect();
    for (i, name) in names.iter().enumerate() {
        let load_id = format!("load_{}", i + 1);
        graph.insert(load_id.clone(), json!({
            "class_type": "LoadImage", "inputs": { "image": p.image }
        }));
        graph.insert(format!("extract_{}", i + 1), json!({
            "class_type": "MiniMaxH3RefModExtract",
            "inputs": {
                "image": [load_id, 0],
                "name": name,
                "vae_dir": p.vae_dir,
                "concept_type": "appearance",
                "description": p.view_descriptions[i],
                "identity": 500,
                "mode": "encode",
                "pool": 16,
                "resolution": 2048
            }
        }));
    }
    graph.insert("merge".to_string(), json!({
        "class_type": "MiniMaxH3RefModsMerge",
        "inputs": {
            "mods_1": [extract_ids[0], 0],
            "mods_2": [extract_ids[1], 0],
            "mods_3": [extract_ids[2], 0],
            "mods_4": [extract_ids[3], 0]
        }
    }));
    graph.insert("apply".to_string(), json!({
        "class_type": "MiniMaxH3RefModApply",
        "inputs": { "mods": ["merge", 0], "vae_dir": p.vae_dir, "strength": p.strength }
    }));
    graph.insert("pipeline".to_string(), json!({
        "class_type": "MiniMaxH3Pipeline",
        "inputs": {
            "minimax_h3_model": "MiniMax-H3",
            "mode": "ref2va",
            "prompt": p.prompt,
            "embed_frames": p.embed_frames,
            "fps": p.fps,
            "height": p.height,
            "image": ["apply", 0],
            "num_frames": p.num_frames,
            "output_subfolder": null,
            "seed": p.seed,
            "split_gpu": p.split_gpu,
            "steps": p.steps,
            "width": p.width
        }
    }));
    graph.insert("save".to_string(), save_video("pipeline", &p.prefix, p.fps));
    Value::Object(graph)
}

/// Video FL2VA — first/last frame + prompt (i2va mode).
pub struct Fl2VaParams {
    pub first_frame: String,
    pub last_frame: String,
    pub prompt: String,
    pub fps: u32,
    pub height: u32,
    pub width: u32,
    pub num_frames: u32,
    pub seed: u32,
    pub steps: u32,
    pub embed_frames: bool,
    pub prefix: String,
}

pub fn fl2va_workflow(p: &Fl2VaParams) -> Value {
    let mut graph = serde_json::Map::new();
    graph.insert("load_first".to_string(), json!({
        "class_type": "LoadImage", "inputs": { "image": p.first_frame }
    }));
    graph.insert("load_last".to_string(), json!({
        "class_type": "LoadImage", "inputs": { "image": p.last_frame }
    }));
    graph.insert("pipeline".to_string(), json!({
        "class_type": "MiniMaxH3Pipeline",
        "inputs": {
            "minimax_h3_model": "MiniMax-H3",
            "mode": "i2va",
            "prompt": p.prompt,
            "image": ["load_first", 0],
            "last_frame": ["load_last", 0],
            "embed_frames": p.embed_frames,
            "fps": p.fps,
            "height": p.height,
            "num_frames": p.num_frames,
            "seed": p.seed,
            "steps": p.steps,
            "width": p.width,
            "output_subfolder": null
        }
    }));
    graph.insert("save".to_string(), save_video("pipeline", &p.prefix, p.fps));
    Value::Object(graph)
}

/// Combined flow — 去背景 → 4 视图 chained, 电影画质 in parallel.
///
/// Single graph: source image → RemoveBackground → MultiViewPose → ImageGrid → SaveImage
/// with the cinematic branch (LoadImage → QwenImagePipeline → SaveImage) in parallel.
pub fn combined_workflow(p: &CinematicParams, multi: &MultiViewParams) -> Value {
    json!({
        "1": { "class_type": "LoadImage", "inputs": { "image": p.image } },
        // Main chain: 去背景 → 4视图
        "2": { "class_type": "RemoveBackground", "inputs": { "image": ["1", 0], "model": "bria-rmbg" } },
        "3": {
            "class_type": "QwenImageMultiViewPose",
            "inputs": {
                "image": ["2", 0],
                "instructions": multi.instructions,
                "qwen_model": multi.model,
                "cfg": multi.cfg,
                "height": multi.height,
                "lora": multi.lora,
                "lora_strength": multi.lora_strength,
                "negative_prompt": multi.negative_prompt,
                "seed": multi.seed,
                "steps": multi.steps,
                "subject_guidance": multi.subject_guidance,
                "width": multi.width
            }
        },
        "4": { "class_type": "ImageGrid", "inputs": { "images": ["3", 0], "columns": 2 } },
        "5": { "class_type": "SaveImage", "inputs": { "filename_prefix": multi.prefix, "images": ["4", 0] } },
        // Parallel branch: 电影画质（参考图，不经过去背景）
        "6": {
            "class_type": "QwenImagePipeline",
            "inputs": {
                "prompt": p.prompt,
                "qwen_model": p.model,
                "cfg": p.cfg,
                "height": p.height,
                "image": ["1", 0],
                "negative_prompt": p.negative_prompt,
                "seed": p.seed,
                "steps": p.steps,
                "width": p.width
            }
        },
        "7": { "class_type": "SaveImage", "inputs": { "filename_prefix": p.prefix, "images": ["6", 0] } }
    })
}

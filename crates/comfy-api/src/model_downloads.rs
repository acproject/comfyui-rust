use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelDownloadEntry {
    pub name: String,
    pub description: String,
    pub category: String,
    pub model_type: String,
    pub urls: Vec<ModelUrl>,
    pub dependencies: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelUrl {
    pub label: String,
    pub url: String,
    pub format: String,
}

pub fn get_model_download_list() -> Vec<ModelDownloadEntry> {
    vec![
        ModelDownloadEntry {
            name: "Stable Diffusion v1.4".into(),
            description: "Stable Diffusion v1.4 原始模型".into(),
            category: "sd".into(),
            model_type: "checkpoints".into(),
            urls: vec![ModelUrl {
                label: "CompVis/stable-diffusion-v-1-4-original".into(),
                url: "https://huggingface.co/CompVis/stable-diffusion-v-1-4-original".into(),
                format: "safetensors/ckpt".into(),
            }],
            dependencies: vec![],
        },
        ModelDownloadEntry {
            name: "Stable Diffusion v1.5".into(),
            description: "Stable Diffusion v1.5".into(),
            category: "sd".into(),
            model_type: "checkpoints".into(),
            urls: vec![ModelUrl {
                label: "runwayml/stable-diffusion-v1-5".into(),
                url: "https://huggingface.co/runwayml/stable-diffusion-v1-5".into(),
                format: "safetensors/ckpt".into(),
            }],
            dependencies: vec![],
        },
        ModelDownloadEntry {
            name: "Stable Diffusion v2.1".into(),
            description: "Stable Diffusion v2.1".into(),
            category: "sd".into(),
            model_type: "checkpoints".into(),
            urls: vec![ModelUrl {
                label: "stabilityai/stable-diffusion-2-1".into(),
                url: "https://huggingface.co/stabilityai/stable-diffusion-2-1".into(),
                format: "safetensors/ckpt".into(),
            }],
            dependencies: vec![],
        },
        ModelDownloadEntry {
            name: "Stable Diffusion 3 Medium".into(),
            description: "SD3 2B medium 模型".into(),
            category: "sd3".into(),
            model_type: "checkpoints".into(),
            urls: vec![ModelUrl {
                label: "stabilityai/stable-diffusion-3-medium".into(),
                url: "https://huggingface.co/stabilityai/stable-diffusion-3-medium".into(),
                format: "safetensors".into(),
            }],
            dependencies: vec!["clip_l".into(), "clip_g".into(), "t5xxl".into()],
        },
        ModelDownloadEntry {
            name: "Stable Diffusion 3.5 Large".into(),
            description: "SD3.5 Large 模型".into(),
            category: "sd3".into(),
            model_type: "checkpoints".into(),
            urls: vec![ModelUrl {
                label: "sd3.5_large.safetensors".into(),
                url: "https://huggingface.co/stabilityai/stable-diffusion-3.5-large/blob/main/sd3.5_large.safetensors".into(),
                format: "safetensors".into(),
            }],
            dependencies: vec!["clip_l".into(), "clip_g".into(), "t5xxl".into()],
        },
        ModelDownloadEntry {
            name: "FLUX.1-dev".into(),
            description: "FLUX.1 dev 模型 (需配合 clip_l + t5xxl + vae 使用)".into(),
            category: "flux".into(),
            model_type: "diffusion_models".into(),
            urls: vec![
                ModelUrl {
                    label: "flux1-dev.safetensors".into(),
                    url: "https://huggingface.co/black-forest-labs/FLUX.1-dev/blob/main/flux1-dev.safetensors".into(),
                    format: "safetensors".into(),
                },
                ModelUrl {
                    label: "FLUX.1-dev-gguf (预转换)".into(),
                    url: "https://huggingface.co/leejet/FLUX.1-dev-gguf".into(),
                    format: "gguf".into(),
                },
            ],
            dependencies: vec!["flux_vae".into(), "clip_l".into(), "t5xxl".into()],
        },
        ModelDownloadEntry {
            name: "FLUX.1-schnell".into(),
            description: "FLUX.1 schnell 模型 (快速推理，4步即可)".into(),
            category: "flux".into(),
            model_type: "diffusion_models".into(),
            urls: vec![
                ModelUrl {
                    label: "flux1-schnell.safetensors".into(),
                    url: "https://huggingface.co/black-forest-labs/FLUX.1-schnell/blob/main/flux1-schnell.safetensors".into(),
                    format: "safetensors".into(),
                },
                ModelUrl {
                    label: "FLUX.1-schnell-gguf (预转换)".into(),
                    url: "https://huggingface.co/leejet/FLUX.1-schnell-gguf".into(),
                    format: "gguf".into(),
                },
            ],
            dependencies: vec!["flux_vae".into(), "clip_l".into(), "t5xxl".into()],
        },
        ModelDownloadEntry {
            name: "FLUX.1-Kontext-dev".into(),
            description: "FLUX.1 Kontext 图像编辑模型".into(),
            category: "flux".into(),
            model_type: "diffusion_models".into(),
            urls: vec![
                ModelUrl {
                    label: "flux1-kontext-dev.safetensors".into(),
                    url: "https://huggingface.co/black-forest-labs/FLUX.1-Kontext-dev/blob/main/flux1-kontext-dev.safetensors".into(),
                    format: "safetensors".into(),
                },
                ModelUrl {
                    label: "FLUX.1-Kontext-dev-GGUF (预转换)".into(),
                    url: "https://huggingface.co/QuantStack/FLUX.1-Kontext-dev-GGUF".into(),
                    format: "gguf".into(),
                },
            ],
            dependencies: vec!["flux_vae".into(), "clip_l".into(), "t5xxl".into()],
        },
        ModelDownloadEntry {
            name: "FLUX.2-dev".into(),
            description: "FLUX.2 dev 模型 (需配合 LLM 使用)".into(),
            category: "flux2".into(),
            model_type: "diffusion_models".into(),
            urls: vec![ModelUrl {
                label: "FLUX.2-dev-gguf".into(),
                url: "https://huggingface.co/city96/FLUX.2-dev-gguf/tree/main".into(),
                format: "gguf".into(),
            }],
            dependencies: vec!["flux2_vae".into(), "mistral_small_3.2_24b".into()],
        },
        ModelDownloadEntry {
            name: "FLUX.2-klein-4B".into(),
            description: "FLUX.2 klein 4B 轻量模型".into(),
            category: "flux2".into(),
            model_type: "diffusion_models".into(),
            urls: vec![
                ModelUrl {
                    label: "FLUX.2-klein-4B".into(),
                    url: "https://huggingface.co/black-forest-labs/FLUX.2-klein-4B".into(),
                    format: "safetensors".into(),
                },
                ModelUrl {
                    label: "FLUX.2-klein-4B-GGUF".into(),
                    url: "https://huggingface.co/leejet/FLUX.2-klein-4B-GGUF/tree/main".into(),
                    format: "gguf".into(),
                },
            ],
            dependencies: vec!["flux2_vae".into(), "qwen3_4b".into()],
        },
        ModelDownloadEntry {
            name: "FLUX.2-klein-9B".into(),
            description: "FLUX.2 klein 9B 模型".into(),
            category: "flux2".into(),
            model_type: "diffusion_models".into(),
            urls: vec![
                ModelUrl {
                    label: "FLUX.2-klein-9B".into(),
                    url: "https://huggingface.co/black-forest-labs/FLUX.2-klein-9B".into(),
                    format: "safetensors".into(),
                },
                ModelUrl {
                    label: "FLUX.2-klein-9B-GGUF".into(),
                    url: "https://huggingface.co/leejet/FLUX.2-klein-9B-GGUF/tree/main".into(),
                    format: "gguf".into(),
                },
            ],
            dependencies: vec!["flux2_vae".into(), "qwen3_8b".into()],
        },
        ModelDownloadEntry {
            name: "Chroma".into(),
            description: "Chroma 模型 (需配合 t5xxl 使用)".into(),
            category: "chroma".into(),
            model_type: "diffusion_models".into(),
            urls: vec![
                ModelUrl {
                    label: "Chroma".into(),
                    url: "https://huggingface.co/lodestones/Chroma".into(),
                    format: "safetensors".into(),
                },
                ModelUrl {
                    label: "Chroma-GGUF (预转换)".into(),
                    url: "https://huggingface.co/silveroxides/Chroma-GGUF".into(),
                    format: "gguf".into(),
                },
            ],
            dependencies: vec!["flux_vae".into(), "t5xxl".into()],
        },
        ModelDownloadEntry {
            name: "Chroma1-Radiance".into(),
            description: "Chroma1 Radiance 模型 (无需 VAE)".into(),
            category: "chroma".into(),
            model_type: "diffusion_models".into(),
            urls: vec![
                ModelUrl {
                    label: "Chroma1-Radiance".into(),
                    url: "https://huggingface.co/lodestones/Chroma1-Radiance/tree/main".into(),
                    format: "safetensors".into(),
                },
                ModelUrl {
                    label: "Chroma1-Radiance-GGUF".into(),
                    url: "https://huggingface.co/silveroxides/Chroma1-Radiance-GGUF/tree/main".into(),
                    format: "gguf".into(),
                },
            ],
            dependencies: vec!["t5xxl".into()],
        },
        ModelDownloadEntry {
            name: "Anima".into(),
            description: "Anima 动画风格模型".into(),
            category: "anima".into(),
            model_type: "diffusion_models".into(),
            urls: vec![
                ModelUrl {
                    label: "Anima".into(),
                    url: "https://huggingface.co/circlestone-labs/Anima/tree/main/split_files/diffusion_models".into(),
                    format: "safetensors".into(),
                },
                ModelUrl {
                    label: "Anima-GGUF".into(),
                    url: "https://huggingface.co/Bedovyy/Anima-GGUF/tree/main".into(),
                    format: "gguf".into(),
                },
            ],
            dependencies: vec!["qwen_image_vae".into(), "qwen3_06b".into()],
        },
        ModelDownloadEntry {
            name: "Qwen-Image".into(),
            description: "Qwen-Image 文字渲染与图像编辑模型".into(),
            category: "qwen_image".into(),
            model_type: "diffusion_models".into(),
            urls: vec![
                ModelUrl {
                    label: "Qwen-Image".into(),
                    url: "https://huggingface.co/Comfy-Org/Qwen-Image_ComfyUI/tree/main/split_files/diffusion_models".into(),
                    format: "safetensors".into(),
                },
                ModelUrl {
                    label: "Qwen-Image-GGUF".into(),
                    url: "https://huggingface.co/QuantStack/Qwen-Image-GGUF/tree/main".into(),
                    format: "gguf".into(),
                },
            ],
            dependencies: vec!["qwen_image_vae".into(), "qwen_2.5_vl_7b".into()],
        },
        ModelDownloadEntry {
            name: "Qwen-Image-Edit".into(),
            description: "Qwen-Image-Edit 图像编辑模型".into(),
            category: "qwen_image".into(),
            model_type: "diffusion_models".into(),
            urls: vec![
                ModelUrl {
                    label: "Qwen-Image-Edit".into(),
                    url: "https://huggingface.co/Comfy-Org/Qwen-Image-Edit_ComfyUI/tree/main/split_files/diffusion_models".into(),
                    format: "safetensors".into(),
                },
                ModelUrl {
                    label: "Qwen-Image-Edit-GGUF".into(),
                    url: "https://huggingface.co/QuantStack/Qwen-Image-Edit-GGUF/tree/main".into(),
                    format: "gguf".into(),
                },
            ],
            dependencies: vec!["qwen_image_vae".into(), "qwen_2.5_vl_7b".into()],
        },
        ModelDownloadEntry {
            name: "Ovis-Image-7B".into(),
            description: "Ovis-Image 7B 模型".into(),
            category: "ovis_image".into(),
            model_type: "diffusion_models".into(),
            urls: vec![
                ModelUrl {
                    label: "Ovis-Image".into(),
                    url: "https://huggingface.co/Comfy-Org/Ovis-Image/tree/main/split_files/diffusion_models".into(),
                    format: "safetensors".into(),
                },
                ModelUrl {
                    label: "Ovis-Image-7B-GGUF".into(),
                    url: "https://huggingface.co/leejet/Ovis-Image-7B-GGUF".into(),
                    format: "gguf".into(),
                },
            ],
            dependencies: vec!["flux_vae".into(), "ovis_2.5".into()],
        },
        ModelDownloadEntry {
            name: "Z-Image".into(),
            description: "Z-Image 模型 (4GB VRAM 即可运行)".into(),
            category: "z_image".into(),
            model_type: "diffusion_models".into(),
            urls: vec![
                ModelUrl {
                    label: "Z-Image".into(),
                    url: "https://huggingface.co/Comfy-Org/z_image/tree/main/split_files/diffusion_models".into(),
                    format: "safetensors".into(),
                },
                ModelUrl {
                    label: "Z-Image-GGUF".into(),
                    url: "https://huggingface.co/unsloth/Z-Image-GGUF/tree/main".into(),
                    format: "gguf".into(),
                },
            ],
            dependencies: vec!["flux_vae".into(), "qwen3_4b".into()],
        },
        ModelDownloadEntry {
            name: "Z-Image-Turbo".into(),
            description: "Z-Image Turbo 快速推理模型".into(),
            category: "z_image".into(),
            model_type: "diffusion_models".into(),
            urls: vec![
                ModelUrl {
                    label: "Z-Image-Turbo".into(),
                    url: "https://huggingface.co/Comfy-Org/z_image_turbo/tree/main/split_files/diffusion_models".into(),
                    format: "safetensors".into(),
                },
                ModelUrl {
                    label: "Z-Image-Turbo-GGUF".into(),
                    url: "https://huggingface.co/leejet/Z-Image-Turbo-GGUF/tree/main".into(),
                    format: "gguf".into(),
                },
            ],
            dependencies: vec!["flux_vae".into(), "qwen3_4b".into()],
        },
        ModelDownloadEntry {
            name: "ERNIE-Image".into(),
            description: "ERNIE-Image 模型".into(),
            category: "ernie_image".into(),
            model_type: "diffusion_models".into(),
            urls: vec![
                ModelUrl {
                    label: "ERNIE-Image".into(),
                    url: "https://huggingface.co/Comfy-Org/ERNIE-Image/tree/main/diffusion_models".into(),
                    format: "safetensors".into(),
                },
                ModelUrl {
                    label: "ERNIE-Image-GGUF".into(),
                    url: "https://huggingface.co/unsloth/ERNIE-Image-GGUF/tree/main".into(),
                    format: "gguf".into(),
                },
            ],
            dependencies: vec!["flux2_vae".into(), "ministral_3b".into()],
        },
        ModelDownloadEntry {
            name: "ERNIE-Image-Turbo".into(),
            description: "ERNIE-Image Turbo 快速推理模型".into(),
            category: "ernie_image".into(),
            model_type: "diffusion_models".into(),
            urls: vec![
                ModelUrl {
                    label: "ERNIE-Image-Turbo".into(),
                    url: "https://huggingface.co/Comfy-Org/ERNIE-Image/tree/main/diffusion_models".into(),
                    format: "safetensors".into(),
                },
                ModelUrl {
                    label: "ERNIE-Image-Turbo-GGUF".into(),
                    url: "https://huggingface.co/unsloth/ERNIE-Image-Turbo-GGUF/tree/main".into(),
                    format: "gguf".into(),
                },
            ],
            dependencies: vec!["flux2_vae".into(), "ministral_3b".into()],
        },
        ModelDownloadEntry {
            name: "Wan2.1 T2V 1.3B".into(),
            description: "Wan2.1 文生视频 1.3B 轻量模型".into(),
            category: "wan".into(),
            model_type: "diffusion_models".into(),
            urls: vec![ModelUrl {
                label: "Wan2.1 T2V 1.3B".into(),
                url: "https://huggingface.co/Comfy-Org/Wan_2.1_ComfyUI_repackaged/tree/main/split_files/diffusion_models".into(),
                format: "safetensors".into(),
            }],
            dependencies: vec!["wan_2.1_vae".into(), "umt5_xxl".into()],
        },
        ModelDownloadEntry {
            name: "Wan2.1 T2V 14B".into(),
            description: "Wan2.1 文生视频 14B 模型".into(),
            category: "wan".into(),
            model_type: "diffusion_models".into(),
            urls: vec![
                ModelUrl {
                    label: "Wan2.1 T2V 14B".into(),
                    url: "https://huggingface.co/Comfy-Org/Wan_2.1_ComfyUI_repackaged/tree/main/split_files/diffusion_models".into(),
                    format: "safetensors".into(),
                },
                ModelUrl {
                    label: "Wan2.1-T2V-14B-gguf".into(),
                    url: "https://huggingface.co/city96/Wan2.1-T2V-14B-gguf/tree/main".into(),
                    format: "gguf".into(),
                },
            ],
            dependencies: vec!["wan_2.1_vae".into(), "umt5_xxl".into()],
        },
        ModelDownloadEntry {
            name: "Wan2.1 I2V 14B 480P".into(),
            description: "Wan2.1 图生视频 14B 480P 模型".into(),
            category: "wan".into(),
            model_type: "diffusion_models".into(),
            urls: vec![
                ModelUrl {
                    label: "Wan2.1 I2V 14B 480P".into(),
                    url: "https://huggingface.co/Comfy-Org/Wan_2.1_ComfyUI_repackaged/tree/main/split_files/diffusion_models".into(),
                    format: "safetensors".into(),
                },
                ModelUrl {
                    label: "Wan2.1-I2V-14B-480P-gguf".into(),
                    url: "https://huggingface.co/city96/Wan2.1-I2V-14B-480P-gguf/tree/main".into(),
                    format: "gguf".into(),
                },
            ],
            dependencies: vec!["wan_2.1_vae".into(), "umt5_xxl".into(), "clip_vision_h".into()],
        },
        ModelDownloadEntry {
            name: "Wan2.2 TI2V 5B".into(),
            description: "Wan2.2 文/图生视频 5B 模型".into(),
            category: "wan".into(),
            model_type: "diffusion_models".into(),
            urls: vec![
                ModelUrl {
                    label: "Wan2.2 TI2V 5B".into(),
                    url: "https://huggingface.co/Comfy-Org/Wan_2.2_ComfyUI_Repackaged/tree/main/split_files/diffusion_models".into(),
                    format: "safetensors".into(),
                },
                ModelUrl {
                    label: "Wan2.2-TI2V-5B-GGUF".into(),
                    url: "https://huggingface.co/QuantStack/Wan2.2-TI2V-5B-GGUF/tree/main".into(),
                    format: "gguf".into(),
                },
            ],
            dependencies: vec!["wan_2.2_vae".into(), "umt5_xxl".into()],
        },
        ModelDownloadEntry {
            name: "Wan2.2 T2V A14B".into(),
            description: "Wan2.2 文生视频 A14B 模型 (双模型)".into(),
            category: "wan".into(),
            model_type: "diffusion_models".into(),
            urls: vec![
                ModelUrl {
                    label: "Wan2.2 T2V A14B".into(),
                    url: "https://huggingface.co/Comfy-Org/Wan_2.2_ComfyUI_Repackaged/tree/main/split_files/diffusion_models".into(),
                    format: "safetensors".into(),
                },
                ModelUrl {
                    label: "Wan2.2-T2V-A14B-GGUF".into(),
                    url: "https://huggingface.co/QuantStack/Wan2.2-T2V-A14B-GGUF/tree/main".into(),
                    format: "gguf".into(),
                },
            ],
            dependencies: vec!["wan_2.1_vae".into(), "umt5_xxl".into()],
        },
        ModelDownloadEntry {
            name: "Wan2.2 I2V A14B".into(),
            description: "Wan2.2 图生视频 A14B 模型 (双模型)".into(),
            category: "wan".into(),
            model_type: "diffusion_models".into(),
            urls: vec![
                ModelUrl {
                    label: "Wan2.2 I2V A14B".into(),
                    url: "https://huggingface.co/Comfy-Org/Wan_2.2_ComfyUI_Repackaged/tree/main/split_files/diffusion_models".into(),
                    format: "safetensors".into(),
                },
                ModelUrl {
                    label: "Wan2.2-I2V-A14B-GGUF".into(),
                    url: "https://huggingface.co/QuantStack/Wan2.2-I2V-A14B-GGUF/tree/main".into(),
                    format: "gguf".into(),
                },
            ],
            dependencies: vec!["wan_2.1_vae".into(), "umt5_xxl".into(), "clip_vision_h".into()],
        },
        ModelDownloadEntry {
            name: "SSD-1B".into(),
            description: "SSD-1B 蒸馏模型 (比 SDXL 快 33%)".into(),
            category: "distilled_sd".into(),
            model_type: "checkpoints".into(),
            urls: vec![
                ModelUrl {
                    label: "SSD-1B-A1111.safetensors".into(),
                    url: "https://huggingface.co/segmind/SSD-1B/resolve/main/SSD-1B-A1111.safetensors".into(),
                    format: "safetensors".into(),
                },
                ModelUrl {
                    label: "SSD-1B-fp8".into(),
                    url: "https://huggingface.co/hassenhamdi/SSD-1B-fp8_e4m3fn/resolve/main/SSD-1B_fp8_e4m3fn.safetensors".into(),
                    format: "safetensors".into(),
                },
            ],
            dependencies: vec![],
        },
        ModelDownloadEntry {
            name: "Segmind-Vega".into(),
            description: "Segmind Vega 蒸馏模型".into(),
            category: "distilled_sd".into(),
            model_type: "checkpoints".into(),
            urls: vec![ModelUrl {
                label: "segmind-vega.safetensors".into(),
                url: "https://huggingface.co/segmind/Segmind-Vega/resolve/main/segmind-vega.safetensors".into(),
                format: "safetensors".into(),
            }],
            dependencies: vec![],
        },
        ModelDownloadEntry {
            name: "FLUX VAE (ae.safetensors)".into(),
            description: "FLUX/FLUX2/Kontext/Chroma 使用的 VAE".into(),
            category: "vae".into(),
            model_type: "vae".into(),
            urls: vec![ModelUrl {
                label: "ae.safetensors".into(),
                url: "https://huggingface.co/black-forest-labs/FLUX.1-dev/blob/main/ae.safetensors".into(),
                format: "safetensors".into(),
            }],
            dependencies: vec![],
        },
        ModelDownloadEntry {
            name: "FLUX.2 VAE".into(),
            description: "FLUX.2 使用的 VAE".into(),
            category: "vae".into(),
            model_type: "vae".into(),
            urls: vec![ModelUrl {
                label: "flux2_ae.safetensors".into(),
                url: "https://huggingface.co/black-forest-labs/FLUX.2-dev/tree/main".into(),
                format: "safetensors".into(),
            }],
            dependencies: vec![],
        },
        ModelDownloadEntry {
            name: "Wan2.1 VAE".into(),
            description: "Wan2.1 和 Wan2.2 (除 TI2V 5B) 使用的 VAE".into(),
            category: "vae".into(),
            model_type: "vae".into(),
            urls: vec![ModelUrl {
                label: "wan_2.1_vae.safetensors".into(),
                url: "https://huggingface.co/Comfy-Org/Wan_2.1_ComfyUI_repackaged/blob/main/split_files/vae/wan_2.1_vae.safetensors".into(),
                format: "safetensors".into(),
            }],
            dependencies: vec![],
        },
        ModelDownloadEntry {
            name: "Wan2.2 VAE".into(),
            description: "Wan2.2 TI2V 5B 专用 VAE".into(),
            category: "vae".into(),
            model_type: "vae".into(),
            urls: vec![ModelUrl {
                label: "wan2.2_vae.safetensors".into(),
                url: "https://huggingface.co/Comfy-Org/Wan_2.2_ComfyUI_Repackaged/blob/main/split_files/vae/wan2.2_vae.safetensors".into(),
                format: "safetensors".into(),
            }],
            dependencies: vec![],
        },
        ModelDownloadEntry {
            name: "Qwen-Image VAE".into(),
            description: "Qwen-Image / Anima 使用的 VAE".into(),
            category: "vae".into(),
            model_type: "vae".into(),
            urls: vec![ModelUrl {
                label: "qwen_image_vae.safetensors".into(),
                url: "https://huggingface.co/Comfy-Org/Qwen-Image_ComfyUI/tree/main/split_files/vae".into(),
                format: "safetensors".into(),
            }],
            dependencies: vec![],
        },
        ModelDownloadEntry {
            name: "CLIP-L".into(),
            description: "CLIP-L 文本编码器 (SDXL/FLUX/SD3 通用)".into(),
            category: "text_encoders".into(),
            model_type: "text_encoders".into(),
            urls: vec![ModelUrl {
                label: "clip_l.safetensors".into(),
                url: "https://huggingface.co/comfyanonymous/flux_text_encoders/blob/main/clip_l.safetensors".into(),
                format: "safetensors".into(),
            }],
            dependencies: vec![],
        },
        ModelDownloadEntry {
            name: "CLIP-G".into(),
            description: "CLIP-G 文本编码器 (SD3/SD3.5 使用)".into(),
            category: "text_encoders".into(),
            model_type: "text_encoders".into(),
            urls: vec![ModelUrl {
                label: "clip_g.safetensors".into(),
                url: "https://huggingface.co/Comfy-Org/stable-diffusion-3.5-fp8/blob/main/text_encoders/clip_g.safetensors".into(),
                format: "safetensors".into(),
            }],
            dependencies: vec![],
        },
        ModelDownloadEntry {
            name: "T5-XXL".into(),
            description: "T5-XXL 文本编码器 (FLUX/SD3/Chroma 通用)".into(),
            category: "text_encoders".into(),
            model_type: "text_encoders".into(),
            urls: vec![
                ModelUrl {
                    label: "t5xxl_fp16.safetensors".into(),
                    url: "https://huggingface.co/comfyanonymous/flux_text_encoders/blob/main/t5xxl_fp16.safetensors".into(),
                    format: "safetensors".into(),
                },
                ModelUrl {
                    label: "t5xxl-gguf".into(),
                    url: "https://huggingface.co/city96/t5-xxl-encoder-gguf/tree/main".into(),
                    format: "gguf".into(),
                },
            ],
            dependencies: vec![],
        },
        ModelDownloadEntry {
            name: "UMT5-XXL".into(),
            description: "UMT5-XXL 文本编码器 (Wan 系列使用)".into(),
            category: "text_encoders".into(),
            model_type: "text_encoders".into(),
            urls: vec![
                ModelUrl {
                    label: "umt5_xxl_fp16.safetensors".into(),
                    url: "https://huggingface.co/Comfy-Org/Wan_2.1_ComfyUI_repackaged/blob/main/split_files/text_encoders/umt5_xxl_fp16.safetensors".into(),
                    format: "safetensors".into(),
                },
                ModelUrl {
                    label: "umt5-xxl-encoder-gguf".into(),
                    url: "https://huggingface.co/city96/umt5-xxl-encoder-gguf/tree/main".into(),
                    format: "gguf".into(),
                },
            ],
            dependencies: vec![],
        },
        ModelDownloadEntry {
            name: "Qwen2.5-VL-7B".into(),
            description: "Qwen2.5 VL 7B 文本编码器 (Qwen-Image 使用)".into(),
            category: "text_encoders".into(),
            model_type: "text_encoders".into(),
            urls: vec![
                ModelUrl {
                    label: "qwen_2.5_vl_7b.safetensors".into(),
                    url: "https://huggingface.co/Comfy-Org/Qwen-Image_ComfyUI/tree/main/split_files/text_encoders".into(),
                    format: "safetensors".into(),
                },
                ModelUrl {
                    label: "Qwen2.5-VL-7B-Instruct-GGUF".into(),
                    url: "https://huggingface.co/mradermacher/Qwen2.5-VL-7B-Instruct-GGUF/tree/main".into(),
                    format: "gguf".into(),
                },
            ],
            dependencies: vec![],
        },
        ModelDownloadEntry {
            name: "Qwen3-0.6B-Base".into(),
            description: "Qwen3 0.6B 文本编码器 (Anima 使用)".into(),
            category: "text_encoders".into(),
            model_type: "text_encoders".into(),
            urls: vec![
                ModelUrl {
                    label: "qwen_3_06b_base.safetensors".into(),
                    url: "https://huggingface.co/circlestone-labs/Anima/tree/main/split_files/text_encoders".into(),
                    format: "safetensors".into(),
                },
                ModelUrl {
                    label: "Qwen3-0.6B-Base-GGUF".into(),
                    url: "https://huggingface.co/mradermacher/Qwen3-0.6B-Base-GGUF/tree/main".into(),
                    format: "gguf".into(),
                },
            ],
            dependencies: vec![],
        },
        ModelDownloadEntry {
            name: "Qwen3-4B".into(),
            description: "Qwen3 4B 文本编码器 (FLUX.2 klein 4B / Z-Image 使用)".into(),
            category: "text_encoders".into(),
            model_type: "text_encoders".into(),
            urls: vec![
                ModelUrl {
                    label: "qwen_3_4b.safetensors".into(),
                    url: "https://huggingface.co/Comfy-Org/flux2-klein-4B/tree/main/split_files/text_encoders".into(),
                    format: "safetensors".into(),
                },
                ModelUrl {
                    label: "Qwen3-4B-GGUF".into(),
                    url: "https://huggingface.co/unsloth/Qwen3-4B-GGUF/tree/main".into(),
                    format: "gguf".into(),
                },
            ],
            dependencies: vec![],
        },
        ModelDownloadEntry {
            name: "Qwen3-8B".into(),
            description: "Qwen3 8B 文本编码器 (FLUX.2 klein 9B 使用)".into(),
            category: "text_encoders".into(),
            model_type: "text_encoders".into(),
            urls: vec![
                ModelUrl {
                    label: "qwen_3_8b.safetensors".into(),
                    url: "https://huggingface.co/Comfy-Org/flux2-klein-9B/tree/main/split_files/text_encoders".into(),
                    format: "safetensors".into(),
                },
                ModelUrl {
                    label: "Qwen3-8B-GGUF".into(),
                    url: "https://huggingface.co/unsloth/Qwen3-8B-GGUF/tree/main".into(),
                    format: "gguf".into(),
                },
            ],
            dependencies: vec![],
        },
        ModelDownloadEntry {
            name: "Mistral-Small-3.2-24B".into(),
            description: "Mistral Small 3.2 24B 文本编码器 (FLUX.2-dev 使用)".into(),
            category: "text_encoders".into(),
            model_type: "text_encoders".into(),
            urls: vec![ModelUrl {
                label: "Mistral-Small-3.2-24B-Instruct-2506-GGUF".into(),
                url: "https://huggingface.co/unsloth/Mistral-Small-3.2-24B-Instruct-2506-GGUF/tree/main".into(),
                format: "gguf".into(),
            }],
            dependencies: vec![],
        },
        ModelDownloadEntry {
            name: "Ministral-3-3B".into(),
            description: "Ministral 3 3B 文本编码器 (ERNIE-Image 使用)".into(),
            category: "text_encoders".into(),
            model_type: "text_encoders".into(),
            urls: vec![
                ModelUrl {
                    label: "ministral-3-3b.safetensors".into(),
                    url: "https://huggingface.co/Comfy-Org/ERNIE-Image/tree/main/text_encoders".into(),
                    format: "safetensors".into(),
                },
                ModelUrl {
                    label: "Ministral-3-3B-Instruct-2512-GGUF".into(),
                    url: "https://huggingface.co/unsloth/Ministral-3-3B-Instruct-2512-GGUF/tree/main".into(),
                    format: "gguf".into(),
                },
            ],
            dependencies: vec![],
        },
        ModelDownloadEntry {
            name: "Ovis-2.5".into(),
            description: "Ovis 2.5 文本编码器 (Ovis-Image 使用)".into(),
            category: "text_encoders".into(),
            model_type: "text_encoders".into(),
            urls: vec![ModelUrl {
                label: "ovis_2.5.safetensors".into(),
                url: "https://huggingface.co/Comfy-Org/Ovis-Image/tree/main/split_files/text_encoders".into(),
                format: "safetensors".into(),
            }],
            dependencies: vec![],
        },
        ModelDownloadEntry {
            name: "CLIP-Vision-H".into(),
            description: "CLIP Vision H (Wan I2V/FLF2V 使用)".into(),
            category: "clip_vision".into(),
            model_type: "clip_vision".into(),
            urls: vec![ModelUrl {
                label: "clip_vision_h.safetensors".into(),
                url: "https://huggingface.co/Comfy-Org/Wan_2.1_ComfyUI_repackaged/blob/main/split_files/clip_vision/clip_vision_h.safetensors".into(),
                format: "safetensors".into(),
            }],
            dependencies: vec![],
        },
        ModelDownloadEntry {
            name: "RealESRGAN x4plus anime".into(),
            description: "RealESRGAN 动漫风格超分辨率模型".into(),
            category: "upscale".into(),
            model_type: "upscale_models".into(),
            urls: vec![ModelUrl {
                label: "RealESRGAN_x4plus_anime_6B.pth".into(),
                url: "https://github.com/xinntao/Real-ESRGAN/releases/download/v0.2.2.4/RealESRGAN_x4plus_anime_6B.pth".into(),
                format: "pth".into(),
            }],
            dependencies: vec![],
        },
        ModelDownloadEntry {
            name: "TAESD".into(),
            description: "TAESD 轻量 VAE (加速 SD 解码)".into(),
            category: "vae_approx".into(),
            model_type: "vae_approx".into(),
            urls: vec![ModelUrl {
                label: "diffusion_pytorch_model.safetensors".into(),
                url: "https://huggingface.co/madebyollin/taesd/blob/main/diffusion_pytorch_model.safetensors".into(),
                format: "safetensors".into(),
            }],
            dependencies: vec![],
        },
        ModelDownloadEntry {
            name: "TAEHV (Wan2.1)".into(),
            description: "TAEHV 轻量 VAE (Wan2.1 / Qwen-Image 使用)".into(),
            category: "vae_approx".into(),
            model_type: "vae_approx".into(),
            urls: vec![ModelUrl {
                label: "taew2_1.safetensors".into(),
                url: "https://github.com/madebyollin/taehv/raw/refs/heads/main/safetensors/taew2_1.safetensors".into(),
                format: "safetensors".into(),
            }],
            dependencies: vec![],
        },
        ModelDownloadEntry {
            name: "TAEHV (Wan2.2 TI2V 5B)".into(),
            description: "TAEHV 轻量 VAE (Wan2.2 TI2V 5B 专用)".into(),
            category: "vae_approx".into(),
            model_type: "vae_approx".into(),
            urls: vec![ModelUrl {
                label: "taew2_2.safetensors".into(),
                url: "https://github.com/madebyollin/taehv/raw/refs/heads/main/safetensors/taew2_2.safetensors".into(),
                format: "safetensors".into(),
            }],
            dependencies: vec![],
        },
        ModelDownloadEntry {
            name: "PhotoMaker v1".into(),
            description: "PhotoMaker 个性化图像生成 (仅 SDXL)".into(),
            category: "photomaker".into(),
            model_type: "photomarker".into(),
            urls: vec![ModelUrl {
                label: "photomaker-v1.safetensors".into(),
                url: "https://huggingface.co/bssrdf/PhotoMaker".into(),
                format: "safetensors".into(),
            }],
            dependencies: vec![],
        },
        ModelDownloadEntry {
            name: "LCM-LoRA SD v1.5".into(),
            description: "LCM-LoRA 加速推理 (SD v1.5)".into(),
            category: "lora".into(),
            model_type: "loras".into(),
            urls: vec![ModelUrl {
                label: "lcm-lora-sdv1-5".into(),
                url: "https://huggingface.co/latent-consistency/lcm-lora-sdv1-5".into(),
                format: "safetensors".into(),
            }],
            dependencies: vec![],
        },
    ]
}

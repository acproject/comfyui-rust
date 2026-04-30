use comfy_inference::error::InferenceError;
use comfy_inference::image::{ImageError, SdImage, SdVideo};
use comfy_inference::params::*;
use comfy_inference::types::*;
use comfy_inference::InferenceBackend;

#[test]
fn test_sd_type_from_c() {
    assert_eq!(SdType::from_c(0), Some(SdType::F32));
    assert_eq!(SdType::from_c(1), Some(SdType::F16));
    assert_eq!(SdType::from_c(30), Some(SdType::BF16));
    assert_eq!(SdType::from_c(41), Some(SdType::Auto));
    assert_eq!(SdType::from_c(99), None);
}

#[test]
fn test_sd_type_to_c() {
    assert_eq!(SdType::F32.to_c(), 0);
    assert_eq!(SdType::F16.to_c(), 1);
    assert_eq!(SdType::Auto.to_c(), 41);
}

#[test]
fn test_sd_type_display() {
    assert_eq!(format!("{}", SdType::F32), "f32");
    assert_eq!(format!("{}", SdType::F16), "f16");
    assert_eq!(format!("{}", SdType::Auto), "auto");
    assert_eq!(format!("{}", SdType::Q4_K), "q4_k");
}

#[test]
fn test_sample_method_from_c() {
    assert_eq!(SampleMethod::from_c(0), Some(SampleMethod::Euler));
    assert_eq!(SampleMethod::from_c(1), Some(SampleMethod::EulerA));
    assert_eq!(SampleMethod::from_c(9), Some(SampleMethod::LCM));
    assert_eq!(SampleMethod::from_c(99), None);
}

#[test]
fn test_sample_method_display() {
    assert_eq!(format!("{}", SampleMethod::EulerA), "euler_a");
    assert_eq!(format!("{}", SampleMethod::DPMPP2M), "dpmpp_2m");
    assert_eq!(format!("{}", SampleMethod::LCM), "lcm");
}

#[test]
fn test_scheduler_from_c() {
    assert_eq!(Scheduler::from_c(0), Some(Scheduler::Discrete));
    assert_eq!(Scheduler::from_c(1), Some(Scheduler::Karras));
    assert_eq!(Scheduler::from_c(99), None);
}

#[test]
fn test_scheduler_display() {
    assert_eq!(format!("{}", Scheduler::Karras), "karras");
    assert_eq!(format!("{}", Scheduler::SgmUniform), "sgm_uniform");
}

#[test]
fn test_prediction_type_from_c() {
    assert_eq!(PredictionType::from_c(0), Some(PredictionType::Eps));
    assert_eq!(PredictionType::from_c(3), Some(PredictionType::Flow));
    assert_eq!(PredictionType::from_c(99), None);
}

#[test]
fn test_lora_apply_mode_from_c() {
    assert_eq!(LoraApplyMode::from_c(0), Some(LoraApplyMode::Auto));
    assert_eq!(LoraApplyMode::from_c(2), Some(LoraApplyMode::AtRuntime));
    assert_eq!(LoraApplyMode::from_c(99), None);
}

#[test]
fn test_cache_mode_from_c() {
    assert_eq!(CacheMode::from_c(0), Some(CacheMode::Disabled));
    assert_eq!(CacheMode::from_c(4), Some(CacheMode::TaylorSeer));
    assert_eq!(CacheMode::from_c(6), Some(CacheMode::Spectrum));
    assert_eq!(CacheMode::from_c(99), None);
}

#[test]
fn test_cache_mode_display() {
    assert_eq!(format!("{}", CacheMode::Disabled), "disabled");
    assert_eq!(format!("{}", CacheMode::EasyCache), "easycache");
    assert_eq!(format!("{}", CacheMode::Spectrum), "spectrum");
}

#[test]
fn test_hires_upscaler_display() {
    assert_eq!(format!("{}", HiresUpscaler::None), "None");
    assert_eq!(format!("{}", HiresUpscaler::Latent), "Latent");
    assert_eq!(format!("{}", HiresUpscaler::Lanczos), "Lanczos");
}

#[test]
fn test_rng_type_from_c() {
    assert_eq!(RngType::from_c(0), Some(RngType::StdDefault));
    assert_eq!(RngType::from_c(1), Some(RngType::Cuda));
    assert_eq!(RngType::from_c(2), Some(RngType::Cpu));
    assert_eq!(RngType::from_c(3), None);
}

#[test]
fn test_sd_image_new() {
    let img = SdImage::new(64, 64, 3);
    assert_eq!(img.width, 64);
    assert_eq!(img.height, 64);
    assert_eq!(img.channel, 3);
    assert_eq!(img.data.len(), 64 * 64 * 3);
    assert!(img.data.iter().all(|&b| b == 0));
}

#[test]
fn test_sd_image_from_raw() {
    let data = vec![128u8; 64 * 64 * 3];
    let img = SdImage::from_raw(64, 64, 3, data).unwrap();
    assert_eq!(img.width, 64);
    assert_eq!(img.height, 64);
    assert_eq!(img.channel, 3);
}

#[test]
fn test_sd_image_from_raw_size_mismatch() {
    let data = vec![128u8; 100];
    let result = SdImage::from_raw(64, 64, 3, data);
    assert!(matches!(result, Err(ImageError::SizeMismatch { .. })));
}

#[test]
fn test_sd_image_rgb() {
    let data = vec![255u8; 32 * 32 * 3];
    let img = SdImage::rgb(32, 32, data).unwrap();
    assert_eq!(img.channel, 3);
}

#[test]
fn test_sd_image_rgba() {
    let data = vec![255u8; 32 * 32 * 4];
    let img = SdImage::rgba(32, 32, data).unwrap();
    assert_eq!(img.channel, 4);
}

#[test]
fn test_sd_image_grayscale() {
    let data = vec![128u8; 32 * 32];
    let img = SdImage::grayscale(32, 32, data).unwrap();
    assert_eq!(img.channel, 1);
}

#[test]
fn test_sd_image_pixel_count() {
    let img = SdImage::new(64, 48, 3);
    assert_eq!(img.pixel_count(), 64 * 48);
}

#[test]
fn test_sd_image_byte_len() {
    let img = SdImage::new(64, 48, 3);
    assert_eq!(img.byte_len(), 64 * 48 * 3);
}

#[test]
fn test_sd_image_is_empty() {
    let img = SdImage::new(0, 0, 3);
    assert!(img.is_empty());
    let img = SdImage::new(1, 1, 3);
    assert!(!img.is_empty());
}

#[test]
fn test_sd_image_display() {
    let img = SdImage::new(64, 48, 3);
    assert_eq!(format!("{}", img), "SdImage(64x48x3)");
}

#[test]
fn test_sd_image_png_roundtrip() {
    let mut img = SdImage::new(8, 8, 3);
    for (i, pixel) in img.data.chunks_exact_mut(3).enumerate() {
        pixel[0] = (i * 30) as u8;
        pixel[1] = (i * 20) as u8;
        pixel[2] = (i * 10) as u8;
    }
    let png_bytes = img.to_png_bytes().unwrap();
    assert!(!png_bytes.is_empty());

    let decoded = SdImage::from_png_bytes(&png_bytes).unwrap();
    assert_eq!(decoded.width, 8);
    assert_eq!(decoded.height, 8);
    assert_eq!(decoded.channel, 3);
    for i in 0..img.data.len() {
        assert_eq!(img.data[i], decoded.data[i]);
    }
}

#[test]
fn test_sd_image_base64_roundtrip() {
    let img = SdImage::new(4, 4, 3);
    let b64 = img.to_base64_png().unwrap();
    assert!(!b64.is_empty());
}

#[test]
fn test_sd_image_serialization() {
    let img = SdImage::new(4, 4, 3);
    let json = serde_json::to_string(&img).unwrap();
    assert!(json.contains("\"width\":4"));
    assert!(json.contains("\"height\":4"));
    assert!(json.contains("\"channel\":3"));

    let decoded: SdImage = serde_json::from_str(&json).unwrap();
    assert_eq!(decoded.width, 4);
    assert_eq!(decoded.height, 4);
    assert_eq!(decoded.channel, 3);
    assert_eq!(decoded.data, img.data);
}

#[test]
fn test_sd_video() {
    let frames = vec![SdImage::new(64, 64, 3); 5];
    let video = SdVideo::new(frames, 24);
    assert_eq!(video.frame_count(), 5);
    assert_eq!(video.fps, 24);
    assert!(!video.is_empty());
}

#[test]
fn test_sd_video_empty() {
    let video = SdVideo::new(vec![], 16);
    assert!(video.is_empty());
    assert_eq!(video.frame_count(), 0);
}

#[test]
fn test_context_config_default() {
    let config = ContextConfig::default();
    assert!(config.model_path.is_none());
    assert!(config.vae_path.is_none());
    assert_eq!(config.n_threads, -1);
    assert_eq!(config.wtype, SdType::Auto);
    assert_eq!(config.rng_type, RngType::Cuda);
    assert!(!config.flash_attn);
    assert!(!config.vae_decode_only);
}

#[test]
fn test_context_config_builder() {
    let config = ContextConfig::new("/models/sd.safetensors")
        .with_vae("/models/vae.safetensors")
        .with_clip_l("/models/clip_l.safetensors")
        .with_threads(8)
        .with_wtype(SdType::Q4_K)
        .with_flash_attn(true)
        .with_offload_to_cpu(true)
        .with_mmap(true)
        .with_vae_decode_only(true);

    assert_eq!(config.model_path, Some("/models/sd.safetensors".to_string()));
    assert_eq!(config.vae_path, Some("/models/vae.safetensors".to_string()));
    assert_eq!(config.clip_l_path, Some("/models/clip_l.safetensors".to_string()));
    assert_eq!(config.n_threads, 8);
    assert_eq!(config.wtype, SdType::Q4_K);
    assert!(config.flash_attn);
    assert!(config.offload_params_to_cpu);
    assert!(config.enable_mmap);
    assert!(config.vae_decode_only);
}

#[test]
fn test_image_gen_params_default() {
    let params = ImageGenParams::default();
    assert!(params.prompt.is_empty());
    assert!(params.negative_prompt.is_empty());
    assert_eq!(params.width, 512);
    assert_eq!(params.height, 512);
    assert_eq!(params.seed, 42);
    assert_eq!(params.batch_count, 1);
    assert!(params.init_image.is_none());
    assert!(!params.is_img2img());
}

#[test]
fn test_image_gen_params_builder() {
    let params = ImageGenParams::new("a beautiful sunset")
        .with_negative_prompt("blurry, low quality")
        .with_dimensions(1024, 768)
        .with_seed(12345)
        .with_sample_steps(30)
        .with_cfg_scale(8.0)
        .with_sample_method(SampleMethod::DPMPP2M)
        .with_scheduler(Scheduler::Karras)
        .with_batch_count(4)
        .with_lora("/models/lora.safetensors", 0.8)
        .with_strength(0.6);

    assert_eq!(params.prompt, "a beautiful sunset");
    assert_eq!(params.negative_prompt, "blurry, low quality");
    assert_eq!(params.width, 1024);
    assert_eq!(params.height, 768);
    assert_eq!(params.seed, 12345);
    assert_eq!(params.sample_params.sample_steps, 30);
    assert_eq!(params.sample_params.guidance.txt_cfg, 8.0);
    assert_eq!(params.sample_params.sample_method, SampleMethod::DPMPP2M);
    assert_eq!(params.sample_params.scheduler, Scheduler::Karras);
    assert_eq!(params.batch_count, 4);
    assert_eq!(params.loras.len(), 1);
    assert_eq!(params.loras[0].path, "/models/lora.safetensors");
    assert!((params.loras[0].multiplier - 0.8).abs() < f32::EPSILON);
    assert!((params.strength - 0.6).abs() < f32::EPSILON);
}

#[test]
fn test_image_gen_params_img2img() {
    let init_img = SdImage::new(512, 512, 3);
    let params = ImageGenParams::new("test").with_init_image(init_img);
    assert!(params.is_img2img());
    assert!(params.init_image.is_some());
}

#[test]
fn test_video_gen_params_default() {
    let params = VideoGenParams::default();
    assert!(params.prompt.is_empty());
    assert_eq!(params.video_frames, 1);
    assert!((params.moe_boundary - 0.875).abs() < f32::EPSILON);
}

#[test]
fn test_video_gen_params_builder() {
    let params = VideoGenParams::new("a dancing person")
        .with_dimensions(512, 512)
        .with_seed(999)
        .with_video_frames(16)
        .with_sample_steps(25)
        .with_cfg_scale(7.5);

    assert_eq!(params.prompt, "a dancing person");
    assert_eq!(params.video_frames, 16);
    assert_eq!(params.seed, 999);
    assert_eq!(params.sample_params.sample_steps, 25);
    assert!((params.sample_params.guidance.txt_cfg - 7.5).abs() < f32::EPSILON);
}

#[test]
fn test_upscale_params() {
    let params = UpscaleParams::new("/models/esrgan.pth");
    assert_eq!(params.esrgan_path, "/models/esrgan.pth");
    assert_eq!(params.upscale_factor, 4);
    assert_eq!(params.tile_size, 128);
}

#[test]
fn test_sample_params_default() {
    let params = SampleParams::default();
    assert_eq!(params.sample_method, SampleMethod::EulerA);
    assert_eq!(params.scheduler, Scheduler::Discrete);
    assert_eq!(params.sample_steps, 20);
    assert!((params.guidance.txt_cfg - 7.0).abs() < f32::EPSILON);
}

#[test]
fn test_guidance_params_default() {
    let params = GuidanceParams::default();
    assert!((params.txt_cfg - 7.0).abs() < f32::EPSILON);
    assert!(params.img_cfg.is_none());
    assert!((params.distilled_guidance - 3.5).abs() < f32::EPSILON);
}

#[test]
fn test_slg_params_default() {
    let params = SlgParams::default();
    assert_eq!(params.layers, vec![7, 8, 9]);
    assert!((params.layer_start - 0.0).abs() < f32::EPSILON);
    assert!((params.layer_end - 1.0).abs() < f32::EPSILON);
}

#[test]
fn test_cache_params_default() {
    let params = CacheParams::default();
    assert_eq!(params.mode, CacheMode::Disabled);
    assert!((params.end_percent - 1.0).abs() < f32::EPSILON);
}

#[test]
fn test_hires_params_default() {
    let params = HiresParams::default();
    assert!(!params.enabled);
    assert_eq!(params.upscaler, HiresUpscaler::Latent);
    assert!((params.scale - 2.0).abs() < f32::EPSILON);
}

#[test]
fn test_tiling_params_default() {
    let params = TilingParams::default();
    assert!(!params.enabled);
    assert!((params.target_overlap - 0.5).abs() < f32::EPSILON);
}

#[test]
fn test_lora_entry() {
    let entry = LoraEntry {
        path: "/models/lora.safetensors".to_string(),
        multiplier: 0.8,
        is_high_noise: false,
    };
    assert_eq!(entry.path, "/models/lora.safetensors");
    assert!((entry.multiplier - 0.8).abs() < f32::EPSILON);
    assert!(!entry.is_high_noise);
}

#[test]
fn test_null_backend() {
    use comfy_inference::NullBackend;
    let backend = NullBackend;
    assert!(!backend.supports_image_generation());
    assert!(!backend.supports_video_generation());

    let result = backend.generate_image(ImageGenParams::new("test"));
    assert!(matches!(result, Err(InferenceError::BackendNotAvailable(_))));

    let result = backend.generate_video(VideoGenParams::new("test"));
    assert!(matches!(result, Err(InferenceError::BackendNotAvailable(_))));
}

#[test]
fn test_backend_capabilities() {
    use comfy_inference::NullBackend;
    let backend = NullBackend;
    let caps = backend.get_capabilities();
    assert!(!caps.supports_image_generation);
    assert!(!caps.supports_video_generation);
}

#[test]
fn test_generation_mode_display() {
    use comfy_inference::GenerationMode;
    assert_eq!(format!("{}", GenerationMode::ImageGeneration), "img_gen");
    assert_eq!(format!("{}", GenerationMode::VideoGeneration), "vid_gen");
}

#[test]
fn test_inference_error_variants() {
    let err = InferenceError::ContextCreationFailed;
    assert_eq!(format!("{}", err), "Context creation failed");

    let err = InferenceError::ModelNotLoaded("sd.safetensors".to_string());
    assert!(format!("{}", err).contains("sd.safetensors"));

    let err = InferenceError::RemoteError {
        status: 404,
        message: "Not found".to_string(),
    };
    assert!(format!("{}", err).contains("404"));
    assert!(format!("{}", err).contains("Not found"));
}

#[test]
fn test_image_gen_params_serialization() {
    let params = ImageGenParams::new("test prompt")
        .with_dimensions(512, 512)
        .with_seed(42);

    let json = serde_json::to_string(&params).unwrap();
    let deserialized: ImageGenParams = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.prompt, "test prompt");
    assert_eq!(deserialized.width, 512);
    assert_eq!(deserialized.height, 512);
    assert_eq!(deserialized.seed, 42);
}

#[test]
fn test_context_config_serialization() {
    let config = ContextConfig::new("/models/sd.safetensors")
        .with_vae("/models/vae.safetensors")
        .with_flash_attn(true);

    let json = serde_json::to_string(&config).unwrap();
    let deserialized: ContextConfig = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.model_path, Some("/models/sd.safetensors".to_string()));
    assert_eq!(deserialized.vae_path, Some("/models/vae.safetensors".to_string()));
    assert!(deserialized.flash_attn);
}

#[test]
fn test_sample_params_serialization() {
    let params = SampleParams {
        guidance: GuidanceParams {
            txt_cfg: 7.5,
            ..Default::default()
        },
        sample_method: SampleMethod::DPMPP2M,
        scheduler: Scheduler::Karras,
        sample_steps: 30,
        ..Default::default()
    };

    let json = serde_json::to_string(&params).unwrap();
    let deserialized: SampleParams = serde_json::from_str(&json).unwrap();
    assert!((deserialized.guidance.txt_cfg - 7.5).abs() < f32::EPSILON);
    assert_eq!(deserialized.sample_method, SampleMethod::DPMPP2M);
    assert_eq!(deserialized.scheduler, Scheduler::Karras);
    assert_eq!(deserialized.sample_steps, 30);
    assert!(deserialized.eta.is_none());
    assert!(deserialized.flow_shift.is_none());
}

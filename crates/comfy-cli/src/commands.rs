//! Subcommand handlers: map parsed CLI arguments onto the existing
//! `comfy-inference` parameter types, invoke the backend, and persist media.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use comfy_inference::image::{SdImage, SdVideo};
use comfy_inference::params::{
    BerniniVideoParams, Gaussian3DParams, H3Mode, H3Params, ImageGenParams, ModelConfig,
    Music3Params,
};
use comfy_inference::{InferenceBackend, InferenceError, PythonBackend};
use serde_json::{Value, json};

use crate::args::{
    AvArgs, Gaussian3dArgs, GlobalOpts, ImageArgs, MusicArgs, Subcommand, VideoArgs,
};
use crate::backend;
use crate::output::{self, OutputEntry};

/// Dispatch result: command name, persisted outputs, arbitrary metadata.
pub type DispatchResult = Result<(String, Vec<OutputEntry>, Value), InferenceError>;

pub fn dispatch(
    backend: &Arc<dyn InferenceBackend>,
    globals: &GlobalOpts,
    command: Subcommand,
) -> DispatchResult {
    match command {
        Subcommand::Image(a) => image(backend, globals, a),
        Subcommand::Music(a) => music(backend, globals, a),
        Subcommand::Video(a) => video(backend, globals, a),
        Subcommand::Av(a) => av(backend, globals, a),
        Subcommand::Gaussian3d(a) => gaussian3d(backend, globals, a),
        Subcommand::Capabilities => capabilities(backend, globals),
        Subcommand::Help(_) | Subcommand::Version => Err(InferenceError::InvalidParameter(
            "help/version are handled before dispatch".to_string(),
        )),
    }
}

// ---------------------------------------------------------------------------
// Shared helpers
// ---------------------------------------------------------------------------

fn load_text(inline: &str, file: &Option<String>, label: &str) -> Result<String, InferenceError> {
    if let Some(path) = file {
        let text = fs::read_to_string(path).map_err(|e| {
            InferenceError::InvalidParameter(format!("cannot read {label} file '{path}': {e}"))
        })?;
        Ok(text)
    } else {
        Ok(inline.to_string())
    }
}

fn load_image(path: &str, flag: &str) -> Result<SdImage, InferenceError> {
    let bytes = fs::read(path).map_err(|e| {
        InferenceError::InvalidParameter(format!("cannot read {flag} '{path}': {e}"))
    })?;
    SdImage::from_image_bytes(&bytes).map_err(|e| {
        InferenceError::InvalidParameter(format!("{flag} '{path}' is not a decodable image: {e}"))
    })
}

fn require_existing(path: &str, flag: &str) -> Result<(), InferenceError> {
    if Path::new(path).exists() {
        Ok(())
    } else {
        Err(InferenceError::InvalidParameter(format!(
            "{flag} path does not exist: '{path}'"
        )))
    }
}

fn unsupported(modal: &str) -> InferenceError {
    InferenceError::BackendNotAvailable(format!(
        "{modal} is not supported by the resolved backend (check `comfy capabilities`)"
    ))
}

// ---------------------------------------------------------------------------
// image
// ---------------------------------------------------------------------------

fn image(
    backend: &Arc<dyn InferenceBackend>,
    globals: &GlobalOpts,
    a: ImageArgs,
) -> DispatchResult {
    if !backend.supports_image_generation() {
        return Err(unsupported("image generation"));
    }
    require_existing(&a.model, "--model")?;
    let prompt = load_text(&a.prompt, &a.prompt_file, "prompt")?;

    let mut params = ImageGenParams::new(prompt)
        .with_model_config(ModelConfig::new().with_model(a.model.clone()))
        .with_dimensions(a.width, a.height)
        .with_seed(a.seed)
        .with_sample_steps(a.steps)
        .with_cfg_scale(a.guidance_scale)
        .with_strength(a.strength);
    if let Some(neg) = a.negative_prompt {
        params.negative_prompt = neg;
    }
    if let Some(path) = &a.init_image {
        params.init_image = Some(load_image(path, "--init-image")?);
    }
    for path in &a.ref_images {
        params.ref_images.push(load_image(path, "--ref-image")?);
    }
    for (i, lora) in a.loras.iter().enumerate() {
        require_existing(lora, "--lora")?;
        let scale = a.lora_scales.get(i).copied().unwrap_or(1.0);
        params = params.with_lora(lora.clone(), scale);
    }

    let images = backend.generate_image(params)?;
    let base = output::prepare_path(
        &a.output,
        &globals.output_dir,
        &format!("image_{}", a.seed),
        "png",
    )?;

    let mut entries = Vec::with_capacity(images.len());
    for (i, img) in images.iter().enumerate() {
        let path = if images.len() == 1 {
            base.clone()
        } else {
            output::indexed_path(&base, i)
        };
        fs::write(&path, img.to_png_bytes()?)?;
        entries.push(output::entry(output::abs_string(&path), "image", "png"));
    }

    let meta = json!({
        "count": images.len(),
        "seed": a.seed,
        "width": a.width,
        "height": a.height,
        "steps": a.steps,
        "guidance_scale": a.guidance_scale,
        "img2img": a.init_image.is_some(),
    });
    Ok(("image".to_string(), entries, meta))
}

// ---------------------------------------------------------------------------
// music (MiniMax-Music3)
// ---------------------------------------------------------------------------

fn music(
    backend: &Arc<dyn InferenceBackend>,
    globals: &GlobalOpts,
    a: MusicArgs,
) -> DispatchResult {
    if !backend.supports_music3() {
        return Err(unsupported("music3 generation"));
    }
    require_existing(&a.model, "--model")?;
    let prompt = load_text(&a.prompt, &a.prompt_file, "prompt")?;
    let lyrics = match (&a.lyrics, &a.lyrics_file) {
        (_, Some(path)) => load_text("", &Some(path.clone()), "lyrics")?,
        (Some(text), None) => text.clone(),
        (None, None) => String::new(),
    };

    let out = output::prepare_path(
        &a.output,
        &globals.output_dir,
        &format!("music_{}", a.seed),
        "wav",
    )?;

    let mut params = Music3Params::new(prompt)
        .with_model_path(a.model.clone())
        .with_lyrics(lyrics)
        .with_duration(a.duration)
        .with_seed(a.seed)
        .with_output_path(output::abs_string(&out));
    if let Some(tokens) = a.max_new_tokens {
        params = params.with_max_new_tokens(tokens);
    }

    let result = backend.generate_music3(params)?;
    let reported = PathBuf::from(&result.output_file);
    let entries = vec![output::entry(
        output::abs_string(&reported),
        "audio",
        "wav",
    )];
    let meta = json!({
        "sample_rate": result.sample_rate,
        "channels": result.channels,
        "duration_sec": result.duration_sec,
        "seed": a.seed,
        "requested_duration_sec": a.duration.clamp(1.0, 360.0),
        "output_path": output::abs_string(&out),
    });
    Ok(("music".to_string(), entries, meta))
}

// ---------------------------------------------------------------------------
// video (Bernini-R)
// ---------------------------------------------------------------------------

fn video(
    backend: &Arc<dyn InferenceBackend>,
    globals: &GlobalOpts,
    a: VideoArgs,
) -> DispatchResult {
    if !backend.supports_bernini_video() {
        return Err(unsupported("Bernini-R video generation"));
    }
    require_existing(&a.model, "--model")?;
    // Same Bernini-R shape rule the FallbackBackend applies at runtime; check
    // it up front so a mismatched model fails before any process is spawned.
    if !PythonBackend::is_bernini_model_dir(&a.model) {
        return Err(InferenceError::InvalidParameter(format!(
            "--model '{}' is not a Bernini-R model directory: expected a directory whose name contains 'bernini' and which contains a config.json",
            a.model
        )));
    }
    if let Some(input) = &a.input_video {
        require_existing(input, "--input-video")?;
    }
    let prompt = load_text(&a.prompt, &a.prompt_file, "prompt")?;

    let out = output::prepare_path(
        &a.output,
        &globals.output_dir,
        &format!("video_{}", a.seed),
        "mp4",
    )?;

    let mut params = BerniniVideoParams::new(
        ModelConfig::new().with_model(a.model.clone()),
        prompt,
    )
    .with_dimensions(a.width, a.height)
    .with_max_image_size(a.max_image_size)
    .with_num_frames(a.frames)
    .with_fps(a.fps)
    .with_seed(a.seed)
    .with_steps(a.steps)
    .with_guidance_mode(a.guidance_mode.clone())
    .with_output_path(output::abs_string(&out));
    if let Some(neg) = &a.negative_prompt {
        params = params.with_negative_prompt(neg.clone());
    }
    if let Some(input) = &a.input_video {
        params = params.with_input_video(input.clone());
    }

    let result = backend.generate_bernini_video(params)?;
    let reported = PathBuf::from(&result.output_file);
    let entries = vec![output::entry(
        output::abs_string(&reported),
        "video",
        "mp4",
    )];
    let meta = json!({
        "task": result.task,
        "guidance_mode": result.guidance_mode,
        "num_frames": result.num_frames,
        "fps": result.fps,
        "seed": result.seed,
        "output_path": output::abs_string(&out),
    });
    Ok(("video".to_string(), entries, meta))
}

// ---------------------------------------------------------------------------
// av (MiniMax-H3)
// ---------------------------------------------------------------------------

fn h3_mode(mode: &str) -> H3Mode {
    match mode {
        "i2va" => H3Mode::I2VA,
        "ref2va" => H3Mode::Ref2VA,
        "mr2va" => H3Mode::MR2VA,
        _ => H3Mode::T2VA,
    }
}

fn av(backend: &Arc<dyn InferenceBackend>, globals: &GlobalOpts, a: AvArgs) -> DispatchResult {
    if !backend.supports_audio_video_generation() {
        return Err(unsupported("audio-video generation"));
    }
    require_existing(&a.model, "--model")?;
    for reference in &a.references {
        require_existing(reference, "--reference")?;
    }
    if !SdVideo::is_ffmpeg_available() {
        return Err(InferenceError::BackendNotAvailable(
            "ffmpeg is required to mux H3 audio/video into MP4 but was not found on PATH"
                .to_string(),
        ));
    }
    let prompt = load_text(&a.prompt, &a.prompt_file, "prompt")?;

    let out = output::prepare_path(
        &a.output,
        &globals.output_dir,
        &format!("av_{}", a.seed),
        "mp4",
    )?;

    let mut params = H3Params::new(prompt)
        .with_steps(a.steps)
        .with_cfg(a.cfg)
        .with_resolution(a.width, a.height)
        .with_num_frames(a.frames)
        .with_fps(a.fps);
    params.mode = h3_mode(&a.mode);
    params.seed = a.seed;
    params.model_path = Some(a.model.clone());
    params.generate_sfx = a.sfx.unwrap_or(true);
    params.generate_bgm = a.bgm.unwrap_or(false);
    if let Some(neg) = &a.negative_prompt {
        params.negative_prompt = neg.clone();
    }
    if let Some(variant) = &a.variant {
        params.variant = Some(variant.clone());
    }
    for path in &a.references {
        params.reference_images.push(load_image(path, "--reference")?);
    }

    let video: SdVideo = backend.generate_av(params)?;
    let fps = video.fps.max(1);
    video.encode_with_ffmpeg(&out, fps, 23)?;

    let entries = vec![output::entry(
        output::abs_string(&out),
        "video",
        "mp4",
    )];
    let meta = json!({
        "mode": a.mode,
        "frames": video.frames.len(),
        "fps": fps,
        "has_audio": video.audio.is_some(),
        "sfx": a.sfx.unwrap_or(true),
        "bgm": a.bgm.unwrap_or(false),
        "seed": a.seed,
    });
    Ok(("av".to_string(), entries, meta))
}

// ---------------------------------------------------------------------------
// gaussian3d (TripoSplat)
// ---------------------------------------------------------------------------

fn gaussian3d(
    backend: &Arc<dyn InferenceBackend>,
    globals: &GlobalOpts,
    a: Gaussian3dArgs,
) -> DispatchResult {
    if !backend.supports_3d_generation() {
        return Err(unsupported("3D gaussian generation"));
    }
    for (path, flag) in [
        (&a.ckpt, "--ckpt"),
        (&a.decoder, "--decoder"),
        (&a.dinov3, "--dinov3"),
        (&a.vae_encoder, "--vae-encoder"),
        (&a.rmbg, "--rmbg"),
        (&a.input_image, "--input-image"),
    ] {
        require_existing(path, flag)?;
    }

    let ext = a.format.clone();
    let out = output::prepare_path(
        &a.output,
        &globals.output_dir,
        &format!("gaussian3d_{}", a.seed),
        &ext,
    )?;

    let model_config = ModelConfig::new()
        .with_model(a.ckpt.clone())
        .with_decoder(a.decoder.clone())
        .with_clip_vision(a.dinov3.clone())
        .with_vae(a.vae_encoder.clone())
        .with_rmbg(a.rmbg.clone());
    let image = load_image(&a.input_image, "--input-image")?;
    let output_format = if a.format == "splat" { 1 } else { 0 };

    let params = Gaussian3DParams::new(model_config)
        .with_input_image(image)
        .with_seed(a.seed)
        .with_steps(a.steps)
        .with_guidance_scale(a.guidance_scale)
        .with_num_gaussians(a.num_gaussians)
        .with_erode_radius(a.erode_radius)
        .with_output_path(output::abs_string(&out))
        .with_output_format(output_format);

    let result = backend.generate_3d_gaussian(params)?;
    let reported = result
        .output_file
        .map(PathBuf::from)
        .unwrap_or_else(|| out.clone());
    let entries = vec![output::entry(
        output::abs_string(&reported),
        "3d_gaussian",
        &ext,
    )];
    let meta = json!({
        "format": ext,
        "num_gaussians": result.num_gaussians,
        "seed": a.seed,
        "output_path": output::abs_string(&out),
    });
    Ok(("gaussian3d".to_string(), entries, meta))
}

// ---------------------------------------------------------------------------
// capabilities
// ---------------------------------------------------------------------------

fn capabilities(
    _backend: &Arc<dyn InferenceBackend>,
    globals: &GlobalOpts,
) -> DispatchResult {
    // Build a probe purely for resolved-path introspection: constructing the
    // Python backend resolves the interpreter/script dir without spawning it.
    let probe = backend::build_python(globals);
    let caps = probe.get_capabilities();
    let script_dir = backend::resolved_script_dir(globals)
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|| "<unresolved>".to_string());
    let meta = json!({
        "interpreter": probe.interpreter(),
        "script_dir": script_dir,
        "device": globals.device,
        "dtype": globals.dtype,
        "capabilities": {
            "image_generation": caps.supports_image_generation,
            "video_generation": caps.supports_video_generation,
            "bernini_video": caps.supports_bernini_video,
            "3d_generation": caps.supports_3d_generation,
            "audio_video_generation": caps.supports_audio_video_generation,
            "context_ir": caps.supports_context_ir,
            "music3": caps.supports_music3,
        }
    });
    Ok(("capabilities".to_string(), Vec::new(), meta))
}

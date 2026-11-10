//! Parser/contract tests for the `comfy` CLI — no GPU or Python required.

use comfy_cli::args::{
    self, AvArgs, Gaussian3dArgs, GlobalOpts, ImageArgs, MusicArgs, Subcommand, VideoArgs,
};

fn tokens(parts: &[&str]) -> Vec<String> {
    parts.iter().map(|s| s.to_string()).collect()
}

fn parse(parts: &[&str]) -> Result<comfy_cli::args::Parsed, args::CliUsage> {
    args::parse(tokens(parts))
}

#[test]
fn no_args_is_a_usage_error() {
    assert!(parse(&[]).is_err());
}

#[test]
fn help_and_version_short_circuit() {
    let parsed = parse(&["--help"]).unwrap();
    assert!(matches!(parsed.command, Subcommand::Help(None)));
    let parsed = parse(&["-h"]).unwrap();
    assert!(matches!(parsed.command, Subcommand::Help(None)));
    let parsed = parse(&["--version"]).unwrap();
    assert!(matches!(parsed.command, Subcommand::Version));
    let parsed = parse(&["-V"]).unwrap();
    assert!(matches!(parsed.command, Subcommand::Version));
    // `help <cmd>` resolves to the per-command help.
    let parsed = parse(&["help", "image"]).unwrap();
    assert!(matches!(parsed.command, Subcommand::Help(ref c) if c.as_deref() == Some("image")));
}

#[test]
fn per_command_help_uses_sentinel_with_exit_zero_semantics() {
    let err = parse(&["image", "--model", "/m", "--prompt", "p", "--help"]).unwrap_err();
    assert_eq!(args::is_help_sentinel(&err), Some("image"));
}

#[test]
fn unknown_subcommand_and_option_are_usage_errors() {
    assert!(parse(&["frobnicate"]).is_err());
    assert!(parse(&["image", "--model", "/m", "--prompt", "p", "--nope"]).is_err());
    assert!(parse(&["--bogus-global", "x", "capabilities"]).is_err());
}

#[test]
fn global_options_are_validated_and_parsed() {
    let parsed = parse(&[
        "--device", "cpu",
        "--dtype", "fp32",
        "--python", "/usr/bin/python3",
        "--script-dir", "/tmp/scripts",
        "--output-dir", "/tmp/out",
        "capabilities",
    ])
    .unwrap();
    let g: GlobalOpts = parsed.globals;
    assert_eq!(g.device, "cpu");
    assert_eq!(g.dtype, "fp32");
    assert_eq!(g.python.as_deref(), Some("/usr/bin/python3"));
    assert_eq!(g.script_dir.as_deref(), Some("/tmp/scripts"));
    assert_eq!(g.output_dir, "/tmp/out");
    assert!(matches!(parsed.command, Subcommand::Capabilities));

    assert!(parse(&["--device", "tpu", "capabilities"]).is_err());
    assert!(parse(&["--dtype", "int4", "capabilities"]).is_err());
}

#[test]
fn inline_equals_form_is_accepted_for_every_value_flag() {
    let parsed = parse(&["image", "--model=/m", "--prompt=hello"]).unwrap();
    match parsed.command {
        Subcommand::Image(ImageArgs { model, prompt, .. }) => {
            assert_eq!(model, "/m");
            assert_eq!(prompt, "hello");
        }
        other => panic!("expected image, got {other:?}"),
    }
    // Numeric and repeatable flags go through the same normalization.
    let parsed = parse(&[
        "image",
        "--model=/m",
        "--prompt=x",
        "--steps=8",
        "--seed=99",
        "--ref-image=/r.png",
    ])
    .unwrap();
    match parsed.command {
        Subcommand::Image(a) => {
            assert_eq!(a.steps, 8);
            assert_eq!(a.seed, 99);
            assert_eq!(a.ref_images, vec!["/r.png"]);
        }
        other => panic!("expected image, got {other:?}"),
    }
    // Global options support the form too.
    let parsed = parse(&["--device=cpu", "--dtype=fp16", "capabilities"]).unwrap();
    assert_eq!(parsed.globals.device, "cpu");
    assert_eq!(parsed.globals.dtype, "fp16");
    // Values may themselves contain '='.
    let parsed = parse(&[
        "image", "--model=/m", "--prompt=a=b&c=d",
    ])
    .unwrap();
    match parsed.command {
        Subcommand::Image(a) => assert_eq!(a.prompt, "a=b&c=d"),
        other => panic!("expected image, got {other:?}"),
    }
}

#[test]
fn image_defaults_and_repeatable_flags() {
    let parsed = parse(&[
        "image",
        "--model", "/models/qwen",
        "--prompt", "a cat",
        "--negative-prompt", "blurry",
        "--ref-image", "/r1.png",
        "--ref-image", "/r2.png",
        "--lora", "/l1.safetensors",
        "--lora", "/l2.safetensors",
        "--lora-scale", "0.8",
        "--init-image", "/in.png",
    ])
    .unwrap();
    match parsed.command {
        Subcommand::Image(a) => {
            assert_eq!(a.model, "/models/qwen");
            assert_eq!(a.prompt, "a cat");
            assert_eq!(a.negative_prompt.as_deref(), Some("blurry"));
            assert_eq!(a.ref_images, vec!["/r1.png", "/r2.png"]);
            assert_eq!(a.loras, vec!["/l1.safetensors", "/l2.safetensors"]);
            assert_eq!(a.lora_scales, vec![0.8]);
            assert_eq!(a.init_image.as_deref(), Some("/in.png"));
            // Documented defaults.
            assert_eq!(a.width, 512);
            assert_eq!(a.height, 512);
            assert_eq!(a.steps, 20);
            assert_eq!(a.guidance_scale, 7.0);
            assert_eq!(a.seed, 42);
            assert_eq!(a.strength, 0.75);
        }
        other => panic!("expected image, got {other:?}"),
    }
}

#[test]
fn image_requires_model_and_prompt() {
    assert!(parse(&["image", "--prompt", "x"]).is_err());
    assert!(parse(&["image", "--model", "/m"]).is_err());
}

#[test]
fn image_rejects_bad_numbers() {
    assert!(parse(&["image", "--model", "/m", "--prompt", "x", "--steps", "lots"]).is_err());
    assert!(parse(&["image", "--model", "/m", "--prompt", "x", "--seed", "1.5"]).is_err());
}

#[test]
fn music_parses_and_enforces_mutually_exclusive_lyrics() {
    let parsed = parse(&[
        "music",
        "--model", "/models/music3",
        "--prompt", "upbeat jazz",
        "--duration", "30",
        "--max-new-tokens", "750",
        "--seed", "7",
    ])
    .unwrap();
    match parsed.command {
        Subcommand::Music(MusicArgs {
            model,
            duration,
            seed,
            max_new_tokens,
            ..
        }) => {
            assert_eq!(model, "/models/music3");
            assert_eq!(duration, 30.0);
            assert_eq!(seed, 7);
            assert_eq!(max_new_tokens, Some(750));
        }
        other => panic!("expected music, got {other:?}"),
    }
    assert!(parse(&[
        "music",
        "--model", "/m",
        "--prompt", "x",
        "--lyrics", "la",
        "--lyrics-file", "/lyr.txt",
    ])
    .is_err());
}

#[test]
fn video_defaults_and_v2v_switch() {
    let parsed = parse(&[
        "video",
        "--model", "/models/bernini",
        "--prompt", "drone shot",
        "--input-video", "/clip.mp4",
        "--guidance-mode", "v2v_apg",
    ])
    .unwrap();
    match parsed.command {
        Subcommand::Video(VideoArgs {
            model,
            input_video,
            guidance_mode,
            width,
            frames,
            fps,
            steps,
            ..
        }) => {
            assert_eq!(model, "/models/bernini");
            assert_eq!(input_video.as_deref(), Some("/clip.mp4"));
            assert_eq!(guidance_mode, "v2v_apg");
            assert_eq!(width, 832);
            assert_eq!(frames, 81);
            assert_eq!(fps, 16);
            assert_eq!(steps, 40);
        }
        other => panic!("expected video, got {other:?}"),
    }
    assert!(parse(&[
        "video", "--model", "/m", "--prompt", "x", "--guidance-mode", "bogus"
    ])
    .is_err());
}

#[test]
fn av_validates_mode_and_references() {
    let parsed = parse(&[
        "av",
        "--model", "/models/h3",
        "--prompt", "rain on a tin roof",
        "--no-bgm",
        "--reference", "/ref.png",
        "--mode", "i2va",
    ])
    .unwrap();
    match parsed.command {
        Subcommand::Av(AvArgs {
            mode,
            references,
            bgm,
            sfx,
            frames,
            cfg,
            ..
        }) => {
            assert_eq!(mode, "i2va");
            assert_eq!(references, vec!["/ref.png"]);
            assert_eq!(bgm, Some(false));
            assert_eq!(sfx, None); // default on is applied by the handler
            assert_eq!(frames, 124);
            assert_eq!(cfg, 7.0);
        }
        other => panic!("expected av, got {other:?}"),
    }
    assert!(parse(&["av", "--model", "/m", "--prompt", "x", "--mode", "sfx"]).is_err());
    // Non-t2va modes require at least one reference image.
    assert!(parse(&["av", "--model", "/m", "--prompt", "x", "--mode", "mr2va"]).is_err());
}

#[test]
fn gaussian3d_requires_all_model_paths_and_validates_format() {
    let ok = [
        "gaussian3d",
        "--ckpt", "/c",
        "--decoder", "/d",
        "--dinov3", "/v",
        "--vae-encoder", "/e",
        "--rmbg", "/r",
        "--input-image", "/i.png",
        "--format", "splat",
    ];
    let parsed = parse(&ok).unwrap();
    match parsed.command {
        Subcommand::Gaussian3d(Gaussian3dArgs {
            ckpt,
            format,
            num_gaussians,
            erode_radius,
            ..
        }) => {
            assert_eq!(ckpt, "/c");
            assert_eq!(format, "splat");
            assert_eq!(num_gaussians, 262144);
            assert_eq!(erode_radius, 1);
        }
        other => panic!("expected gaussian3d, got {other:?}"),
    }

    let mut missing = ok.to_vec();
    missing.retain(|s| *s != "--rmbg" && *s != "/r");
    assert!(parse(&missing).is_err());

    let mut bad_format = ok.to_vec();
    let last = bad_format.len() - 1;
    bad_format[last] = "obj";
    assert!(parse(&bad_format).is_err());
}

#[test]
fn trailing_tokens_are_rejected() {
    assert!(parse(&["capabilities", "extra"]).is_err());
}

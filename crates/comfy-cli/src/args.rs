//! Hand-rolled command-line parser (the workspace has no clap dependency;
//! this mirrors the minimal parser style used by comfy-mcp).
//!
//! Contract:
//! - global options must appear before the subcommand name;
//! - flags accept both `--flag value` and `--flag=value`;
//! - repeatable flags (`--ref-image`, `--lora`, ...) collect into `Vec`s;
//! - any usage problem returns [`CliUsage`], which `main` maps to exit code 2.

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

pub const HELP: &str = r#"comfy — ComfyUI-Rust multimedia command-line interface

Generate image / music / video / audio-video / 3D resources through the same
inference backends used by comfy-server, without running the server.

USAGE:
    comfy [GLOBAL OPTIONS] <COMMAND> [COMMAND OPTIONS]

GLOBAL OPTIONS:
    --python PATH           Python interpreter for the HF fallback scripts
    --script-dir DIR        Directory containing img_generate.py and friends
    --device DEVICE         auto | cuda | cpu              [default: auto]
    --dtype DTYPE           auto | fp16 | bf16 | fp32      [default: auto]
    --sd-cli PATH           Also wrap an sd-cli primary backend (FallbackBackend)
    --output-dir DIR        Default directory for generated files
                            [env COMFY_OUTPUT_DIR, fallback: output/cli]
    -h, --help              Show this help
    -V, --version           Print version

COMMANDS:
    image         Text-to-image / image-to-image / instruction-edit (PNG)
    music         MiniMax-Music3 text-to-music (WAV)
    video         Bernini-R text/image-to-video (MP4)
    av            MiniMax-H3 joint audio-video generation (MP4)
    gaussian3d    TripoSplat image-to-3D Gaussian splats (PLY/SPLAT)
    capabilities  Report backend capabilities as JSON (no model loading)

Run `comfy <COMMAND> --help` for command-specific options.

OUTPUT CONTRACT:
    stdout — exactly one JSON object: {"ok":true,...} on success,
             {"ok":false,"error":...} on failure
    stderr — human-readable logs
    exit   — 0 success, 1 runtime/inference failure, 2 usage error
"#;

#[derive(Debug, Clone, PartialEq)]
pub struct CliUsage(pub String);

impl std::fmt::Display for CliUsage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for CliUsage {}

fn usage(msg: impl Into<String>) -> CliUsage {
    CliUsage(msg.into())
}

#[derive(Debug, Clone, PartialEq)]
pub struct GlobalOpts {
    pub python: Option<String>,
    pub script_dir: Option<String>,
    pub device: String,
    pub dtype: String,
    pub sd_cli: Option<String>,
    pub output_dir: String,
}

impl Default for GlobalOpts {
    fn default() -> Self {
        let output_dir = match std::env::var("COMFY_OUTPUT_DIR") {
            Ok(dir) if !dir.is_empty() => format!("{}/cli", dir.trim_end_matches('/')),
            _ => "output/cli".to_string(),
        };
        Self {
            python: None,
            script_dir: None,
            device: "auto".to_string(),
            dtype: "auto".to_string(),
            sd_cli: None,
            output_dir,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct ImageArgs {
    pub model: String,
    pub prompt: String,
    pub prompt_file: Option<String>,
    pub negative_prompt: Option<String>,
    pub width: i32,
    pub height: i32,
    pub steps: i32,
    pub guidance_scale: f32,
    pub seed: i64,
    pub init_image: Option<String>,
    pub strength: f32,
    pub ref_images: Vec<String>,
    pub loras: Vec<String>,
    pub lora_scales: Vec<f32>,
    pub output: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct MusicArgs {
    pub model: String,
    pub prompt: String,
    pub prompt_file: Option<String>,
    pub lyrics: Option<String>,
    pub lyrics_file: Option<String>,
    pub duration: f64,
    pub seed: i64,
    pub max_new_tokens: Option<i32>,
    pub output: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct VideoArgs {
    pub model: String,
    pub prompt: String,
    pub prompt_file: Option<String>,
    pub negative_prompt: Option<String>,
    pub width: i32,
    pub height: i32,
    pub frames: i32,
    pub fps: i32,
    pub steps: i32,
    pub seed: i64,
    pub max_image_size: i32,
    pub input_video: Option<String>,
    pub guidance_mode: String,
    pub output: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct AvArgs {
    pub model: String,
    pub mode: String,
    pub prompt: String,
    pub prompt_file: Option<String>,
    pub negative_prompt: Option<String>,
    pub width: i32,
    pub height: i32,
    pub frames: i32,
    pub fps: i32,
    pub steps: i32,
    pub cfg: f64,
    pub seed: i64,
    pub references: Vec<String>,
    pub sfx: Option<bool>,
    pub bgm: Option<bool>,
    pub variant: Option<String>,
    pub output: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Gaussian3dArgs {
    pub ckpt: String,
    pub decoder: String,
    pub dinov3: String,
    pub vae_encoder: String,
    pub rmbg: String,
    pub input_image: String,
    pub steps: i32,
    pub guidance_scale: f32,
    pub seed: i64,
    pub num_gaussians: i32,
    pub erode_radius: i32,
    pub format: String,
    pub output: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Subcommand {
    Help(Option<String>),
    Version,
    Capabilities,
    Image(ImageArgs),
    Music(MusicArgs),
    Video(VideoArgs),
    Av(AvArgs),
    Gaussian3d(Gaussian3dArgs),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Parsed {
    pub globals: GlobalOpts,
    pub command: Subcommand,
}

/// Token reader supporting `--flag value` and `--flag=value`.
struct Reader<'a> {
    tokens: Vec<&'a str>,
    pos: usize,
}

impl<'a> Reader<'a> {
    fn new(tokens: Vec<&'a str>) -> Self {
        Self { tokens, pos: 0 }
    }

    fn peek(&self) -> Option<&'a str> {
        self.tokens.get(self.pos).copied()
    }

    fn next_token(&mut self) -> Option<&'a str> {
        let t = self.tokens.get(self.pos).copied();
        if t.is_some() {
            self.pos += 1;
        }
        t
    }

    /// Split an inline `--flag=value` token, otherwise consume the next token.
    fn value(&mut self, token: &'a str, flag: &str) -> Result<String, CliUsage> {
        if let Some(inline) = token.strip_prefix(&format!("{flag}=")) {
            return Ok(inline.to_string());
        }
        match self.next_token() {
            Some(v) if !v.starts_with('-') => Ok(v.to_string()),
            Some(other) => Err(usage(format!("missing value for {flag} (got '{other}')"))),
            None => Err(usage(format!("missing value for {flag}"))),
        }
    }

    fn done(&self) -> bool {
        self.pos >= self.tokens.len()
    }
}

fn parse_i32(flag: &str, raw: &str) -> Result<i32, CliUsage> {
    raw.parse::<i32>()
        .map_err(|_| usage(format!("invalid integer for {flag}: '{raw}'")))
}

fn parse_i64(flag: &str, raw: &str) -> Result<i64, CliUsage> {
    raw.parse::<i64>()
        .map_err(|_| usage(format!("invalid integer for {flag}: '{raw}'")))
}

fn parse_f32(flag: &str, raw: &str) -> Result<f32, CliUsage> {
    raw.parse::<f32>()
        .map_err(|_| usage(format!("invalid number for {flag}: '{raw}'")))
}

fn parse_f64(flag: &str, raw: &str) -> Result<f64, CliUsage> {
    raw.parse::<f64>()
        .map_err(|_| usage(format!("invalid number for {flag}: '{raw}'")))
}

fn parse_globals(reader: &mut Reader) -> Result<GlobalOpts, CliUsage> {
    let mut globals = GlobalOpts::default();
    while let Some(token) = reader.peek() {
        if !token.starts_with('-') {
            break; // subcommand name
        }
        let token = reader.next_token().expect("peeked token exists");
        match token {
            "-h" | "--help" => return Err(usage("__help__:")),
            "-V" | "--version" => return Err(usage("__version__")),
            "--python" => globals.python = Some(reader.value(token, "--python")?),
            "--script-dir" => globals.script_dir = Some(reader.value(token, "--script-dir")?),
            "--device" => {
                let v = reader.value(token, "--device")?;
                if !matches!(v.as_str(), "auto" | "cuda" | "cpu") {
                    return Err(usage(format!("--device must be auto|cuda|cpu, got '{v}'")));
                }
                globals.device = v;
            }
            "--dtype" => {
                let v = reader.value(token, "--dtype")?;
                if !matches!(v.as_str(), "auto" | "fp16" | "bf16" | "fp32") {
                    return Err(usage(format!(
                        "--dtype must be auto|fp16|bf16|fp32, got '{v}'"
                    )));
                }
                globals.dtype = v;
            }
            "--sd-cli" => globals.sd_cli = Some(reader.value(token, "--sd-cli")?),
            "--output-dir" => globals.output_dir = reader.value(token, "--output-dir")?,
            other => return Err(usage(format!("unknown global option: {other}"))),
        }
    }
    Ok(globals)
}

pub fn parse<I>(args: I) -> Result<Parsed, CliUsage>
where
    I: IntoIterator<Item = String>,
{
    let raw: Vec<String> = args.into_iter().collect();
    // Normalize every `--flag=value` token into two logical tokens so all
    // value flags support both spellings without per-flag special arms.
    // Split on the first '=' so values may themselves contain '='.
    let mut owned: Vec<String> = Vec::with_capacity(raw.len());
    for token in raw {
        if token.starts_with("--") {
            if let Some(eq) = token.find('=') {
                let (key, value) = token.split_at(eq);
                owned.push(key.to_string());
                owned.push(value[1..].to_string());
                continue;
            }
        }
        owned.push(token);
    }
    let tokens: Vec<&str> = owned.iter().map(|s| s.as_str()).collect();
    let mut reader = Reader::new(tokens);

    // Leading help/version flags act on the whole program.
    if let Some(first) = reader.peek() {
        if first == "-h" || first == "--help" {
            reader.next_token();
            return Ok(Parsed {
                globals: GlobalOpts::default(),
                command: Subcommand::Help(None),
            });
        }
        if first == "-V" || first == "--version" {
            reader.next_token();
            return Ok(Parsed {
                globals: GlobalOpts::default(),
                command: Subcommand::Version,
            });
        }
    }

    let globals = match parse_globals(&mut reader) {
        Ok(g) => g,
        // `--help` / `--version` interleaved with other global options.
        Err(CliUsage(ref sentinel)) if sentinel == "__help__:" => {
            return Ok(Parsed {
                globals: GlobalOpts::default(),
                command: Subcommand::Help(None),
            });
        }
        Err(CliUsage(ref sentinel)) if sentinel == "__version__" => {
            return Ok(Parsed {
                globals: GlobalOpts::default(),
                command: Subcommand::Version,
            });
        }
        Err(e) => return Err(e),
    };

    let command_name = reader
        .next_token()
        .ok_or_else(|| usage("missing subcommand; run `comfy --help` for usage".to_string()))?;

    let command = match command_name {
        "help" => Subcommand::Help(reader.next_token().map(str::to_string)),
        "version" => Subcommand::Version,
        "capabilities" => parse_capabilities(&mut reader)?,
        "image" => Subcommand::Image(parse_image(&mut reader)?),
        "music" => Subcommand::Music(parse_music(&mut reader)?),
        "video" => Subcommand::Video(parse_video(&mut reader)?),
        "av" => Subcommand::Av(parse_av(&mut reader)?),
        "gaussian3d" => Subcommand::Gaussian3d(parse_gaussian3d(&mut reader)?),
        other => {
            return Err(usage(format!(
                "unknown subcommand '{other}'; run `comfy --help` for usage"
            )));
        }
    };

    if !reader.done() {
        return Err(usage(format!(
            "unexpected trailing argument: '{}'",
            reader.peek().unwrap_or("")
        )));
    }
    Ok(Parsed { globals, command })
}

fn parse_capabilities(reader: &mut Reader) -> Result<Subcommand, CliUsage> {
    while let Some(token) = reader.next_token() {
        match token {
            "-h" | "--help" => return Ok(Subcommand::Help(Some("capabilities".to_string()))),
            _ => return Err(usage(format!("capabilities accepts no arguments (got '{token}')"))),
        }
    }
    Ok(Subcommand::Capabilities)
}

fn parse_image(reader: &mut Reader) -> Result<ImageArgs, CliUsage> {
    let mut a = ImageArgs {
        strength: 0.75,
        width: 512,
        height: 512,
        steps: 20,
        guidance_scale: 7.0,
        seed: 42,
        ..Default::default()
    };
    while let Some(token) = reader.next_token() {
        match token {
            "-h" | "--help" => return Err(help_sentinel("image")),
            "--model" => a.model = reader.value(token, "--model")?,
            "--prompt" => a.prompt = reader.value(token, "--prompt")?,
            "--prompt-file" => a.prompt_file = Some(reader.value(token, "--prompt-file")?),
            "--negative-prompt" => a.negative_prompt = Some(reader.value(token, "--negative-prompt")?),
            "--width" => a.width = parse_i32("--width", &reader.value(token, "--width")?)?,
            "--height" => a.height = parse_i32("--height", &reader.value(token, "--height")?)?,
            "--steps" => a.steps = parse_i32("--steps", &reader.value(token, "--steps")?)?,
            "--guidance-scale" => {
                a.guidance_scale = parse_f32("--guidance-scale", &reader.value(token, "--guidance-scale")?)?
            }
            "--seed" => a.seed = parse_i64("--seed", &reader.value(token, "--seed")?)?,
            "--init-image" => a.init_image = Some(reader.value(token, "--init-image")?),
            "--strength" => a.strength = parse_f32("--strength", &reader.value(token, "--strength")?)?,
            "--ref-image" => a.ref_images.push(reader.value(token, "--ref-image")?),
            "--lora" => a.loras.push(reader.value(token, "--lora")?),
            "--lora-scale" => {
                a.lora_scales.push(parse_f32("--lora-scale", &reader.value(token, "--lora-scale")?)?)
            }
            "--output" => a.output = Some(reader.value(token, "--output")?),
            other => return Err(usage(format!("image: unknown option '{other}'"))),
        }
    }
    if a.model.is_empty() {
        return Err(usage("image: --model PATH is required"));
    }
    if a.prompt.is_empty() && a.prompt_file.is_none() {
        return Err(usage("image: --prompt TEXT or --prompt-file PATH is required"));
    }
    Ok(a)
}

fn parse_music(reader: &mut Reader) -> Result<MusicArgs, CliUsage> {
    let mut a = MusicArgs {
        duration: 60.0,
        ..Default::default()
    };
    while let Some(token) = reader.next_token() {
        match token {
            "-h" | "--help" => return Err(help_sentinel("music")),
            "--model" => a.model = reader.value(token, "--model")?,
            "--prompt" => a.prompt = reader.value(token, "--prompt")?,
            "--prompt-file" => a.prompt_file = Some(reader.value(token, "--prompt-file")?),
            "--lyrics" => a.lyrics = Some(reader.value(token, "--lyrics")?),
            "--lyrics-file" => a.lyrics_file = Some(reader.value(token, "--lyrics-file")?),
            "--duration" => a.duration = parse_f64("--duration", &reader.value(token, "--duration")?)?,
            "--seed" => a.seed = parse_i64("--seed", &reader.value(token, "--seed")?)?,
            "--max-new-tokens" => {
                a.max_new_tokens = Some(parse_i32("--max-new-tokens", &reader.value(token, "--max-new-tokens")?)?)
            }
            "--output" => a.output = Some(reader.value(token, "--output")?),
            other => return Err(usage(format!("music: unknown option '{other}'"))),
        }
    }
    if a.model.is_empty() {
        return Err(usage("music: --model PATH is required"));
    }
    if a.prompt.is_empty() && a.prompt_file.is_none() {
        return Err(usage("music: --prompt TEXT or --prompt-file PATH is required"));
    }
    if a.lyrics.is_some() && a.lyrics_file.is_some() {
        return Err(usage("music: --lyrics and --lyrics-file are mutually exclusive"));
    }
    Ok(a)
}

fn parse_video(reader: &mut Reader) -> Result<VideoArgs, CliUsage> {
    let mut a = VideoArgs {
        width: 832,
        height: 480,
        frames: 81,
        fps: 16,
        steps: 40,
        seed: 42,
        max_image_size: 848,
        guidance_mode: "auto".to_string(),
        ..Default::default()
    };
    while let Some(token) = reader.next_token() {
        match token {
            "-h" | "--help" => return Err(help_sentinel("video")),
            "--model" => a.model = reader.value(token, "--model")?,
            "--prompt" => a.prompt = reader.value(token, "--prompt")?,
            "--prompt-file" => a.prompt_file = Some(reader.value(token, "--prompt-file")?),
            "--negative-prompt" => a.negative_prompt = Some(reader.value(token, "--negative-prompt")?),
            "--width" => a.width = parse_i32("--width", &reader.value(token, "--width")?)?,
            "--height" => a.height = parse_i32("--height", &reader.value(token, "--height")?)?,
            "--frames" => a.frames = parse_i32("--frames", &reader.value(token, "--frames")?)?,
            "--fps" => a.fps = parse_i32("--fps", &reader.value(token, "--fps")?)?,
            "--steps" => a.steps = parse_i32("--steps", &reader.value(token, "--steps")?)?,
            "--seed" => a.seed = parse_i64("--seed", &reader.value(token, "--seed")?)?,
            "--max-image-size" => {
                a.max_image_size = parse_i32("--max-image-size", &reader.value(token, "--max-image-size")?)?
            }
            "--input-video" => a.input_video = Some(reader.value(token, "--input-video")?),
            "--guidance-mode" => {
                let v = reader.value(token, "--guidance-mode")?;
                if !matches!(v.as_str(), "auto" | "t2v_apg" | "v2v_apg") {
                    return Err(usage(
                        "--guidance-mode must be auto|t2v_apg|v2v_apg".to_string(),
                    ));
                }
                a.guidance_mode = v;
            }
            "--output" => a.output = Some(reader.value(token, "--output")?),
            other => return Err(usage(format!("video: unknown option '{other}'"))),
        }
    }
    if a.model.is_empty() {
        return Err(usage("video: --model PATH is required"));
    }
    if a.prompt.is_empty() && a.prompt_file.is_none() {
        return Err(usage("video: --prompt TEXT or --prompt-file PATH is required"));
    }
    Ok(a)
}

fn parse_av(reader: &mut Reader) -> Result<AvArgs, CliUsage> {
    let mut a = AvArgs {
        mode: "t2va".to_string(),
        width: 832,
        height: 480,
        frames: 124,
        fps: 24,
        steps: 50,
        cfg: 7.0,
        seed: 42,
        ..Default::default()
    };
    while let Some(token) = reader.next_token() {
        match token {
            "-h" | "--help" => return Err(help_sentinel("av")),
            "--model" => a.model = reader.value(token, "--model")?,
            "--mode" => {
                let v = reader.value(token, "--mode")?;
                if !matches!(v.as_str(), "t2va" | "i2va" | "ref2va" | "mr2va") {
                    return Err(usage(
                        "--mode must be t2va|i2va|ref2va|mr2va (sfx/audio-only are Python-unsupported)"
                            .to_string(),
                    ));
                }
                a.mode = v;
            }
            "--prompt" => a.prompt = reader.value(token, "--prompt")?,
            "--prompt-file" => a.prompt_file = Some(reader.value(token, "--prompt-file")?),
            "--negative-prompt" => a.negative_prompt = Some(reader.value(token, "--negative-prompt")?),
            "--width" => a.width = parse_i32("--width", &reader.value(token, "--width")?)?,
            "--height" => a.height = parse_i32("--height", &reader.value(token, "--height")?)?,
            "--frames" => a.frames = parse_i32("--frames", &reader.value(token, "--frames")?)?,
            "--fps" => a.fps = parse_i32("--fps", &reader.value(token, "--fps")?)?,
            "--steps" => a.steps = parse_i32("--steps", &reader.value(token, "--steps")?)?,
            "--cfg" => a.cfg = parse_f64("--cfg", &reader.value(token, "--cfg")?)?,
            "--seed" => a.seed = parse_i64("--seed", &reader.value(token, "--seed")?)?,
            "--reference" => a.references.push(reader.value(token, "--reference")?),
            "--sfx" => a.sfx = Some(true),
            "--no-sfx" => a.sfx = Some(false),
            "--bgm" => a.bgm = Some(true),
            "--no-bgm" => a.bgm = Some(false),
            "--variant" => {
                let v = reader.value(token, "--variant")?;
                if !matches!(v.as_str(), "fl2va" | "ref2va") {
                    return Err(usage("--variant must be fl2va|ref2va".to_string()));
                }
                a.variant = Some(v);
            }
            "--output" => a.output = Some(reader.value(token, "--output")?),
            other => return Err(usage(format!("av: unknown option '{other}'"))),
        }
    }
    if a.model.is_empty() {
        return Err(usage("av: --model PATH is required"));
    }
    if a.prompt.is_empty() && a.prompt_file.is_none() {
        return Err(usage("av: --prompt TEXT or --prompt-file PATH is required"));
    }
    if a.mode != "t2va" && a.references.is_empty() {
        return Err(usage(format!(
            "av: mode '{}' requires at least one --reference image",
            a.mode
        )));
    }
    Ok(a)
}

fn parse_gaussian3d(reader: &mut Reader) -> Result<Gaussian3dArgs, CliUsage> {
    let mut a = Gaussian3dArgs {
        steps: 20,
        guidance_scale: 3.0,
        seed: 42,
        num_gaussians: 262144,
        erode_radius: 1,
        format: "ply".to_string(),
        ..Default::default()
    };
    while let Some(token) = reader.next_token() {
        match token {
            "-h" | "--help" => return Err(help_sentinel("gaussian3d")),
            "--ckpt" => a.ckpt = reader.value(token, "--ckpt")?,
            "--decoder" => a.decoder = reader.value(token, "--decoder")?,
            "--dinov3" => a.dinov3 = reader.value(token, "--dinov3")?,
            "--vae-encoder" => a.vae_encoder = reader.value(token, "--vae-encoder")?,
            "--rmbg" => a.rmbg = reader.value(token, "--rmbg")?,
            "--input-image" => a.input_image = reader.value(token, "--input-image")?,
            "--steps" => a.steps = parse_i32("--steps", &reader.value(token, "--steps")?)?,
            "--guidance-scale" => {
                a.guidance_scale = parse_f32("--guidance-scale", &reader.value(token, "--guidance-scale")?)?
            }
            "--seed" => a.seed = parse_i64("--seed", &reader.value(token, "--seed")?)?,
            "--num-gaussians" => {
                a.num_gaussians = parse_i32("--num-gaussians", &reader.value(token, "--num-gaussians")?)?
            }
            "--erode-radius" => {
                a.erode_radius = parse_i32("--erode-radius", &reader.value(token, "--erode-radius")?)?
            }
            "--format" => {
                let v = reader.value(token, "--format")?;
                if !matches!(v.as_str(), "ply" | "splat") {
                    return Err(usage("--format must be ply|splat".to_string()));
                }
                a.format = v;
            }
            "--output" => a.output = Some(reader.value(token, "--output")?),
            other => return Err(usage(format!("gaussian3d: unknown option '{other}'"))),
        }
    }
    for (flag, value) in [
        ("--ckpt", &a.ckpt),
        ("--decoder", &a.decoder),
        ("--dinov3", &a.dinov3),
        ("--vae-encoder", &a.vae_encoder),
        ("--rmbg", &a.rmbg),
        ("--input-image", &a.input_image),
    ] {
        if value.is_empty() {
            return Err(usage(format!("gaussian3d: {flag} PATH is required")));
        }
    }
    Ok(a)
}

/// Per-command `--help` is routed through a usage error carrying a sentinel
/// prefix that `run` recognizes instead of treating it as a failure.
fn help_sentinel(command: &str) -> CliUsage {
    usage(format!("__help__:{command}"))
}

pub fn is_help_sentinel(err: &CliUsage) -> Option<&str> {
    err.0.strip_prefix("__help__:")
}

pub const IMAGE_HELP: &str = r#"comfy image — text-to-image / img2img / instruction-edit (PNG)

USAGE:
    comfy [GLOBAL OPTIONS] image --model PATH --prompt TEXT [OPTIONS]

REQUIRED:
    --model PATH            HF/diffusers pipeline directory (or checkpoint)
    --prompt TEXT           Prompt text (or use --prompt-file)

OPTIONS:
    --prompt-file PATH      Read prompt from a UTF-8 text file
    --negative-prompt TEXT
    --width N               [default: 512]
    --height N              [default: 512]
    --steps N               [default: 20]
    --guidance-scale F      [default: 7.0]
    --seed N                [default: 42]
    --init-image PATH       Input image for img2img / edit pipelines
    --strength F            img2img strength [default: 0.75]
    --ref-image PATH        Reference image (repeatable; edit pipelines)
    --lora PATH             LoRA weights (repeatable)
    --lora-scale F          Matching LoRA strength (repeatable, default 1.0)
    --output PATH           Destination .png (default: <output-dir>/image_<seed>.png)
"#;

pub const MUSIC_HELP: &str = r#"comfy music — MiniMax-Music3 text-to-music (32 kHz stereo WAV)

USAGE:
    comfy [GLOBAL OPTIONS] music --model PATH --prompt TEXT [OPTIONS]

REQUIRED:
    --model PATH            Music3 modular repository (modular_model_index.json)
    --prompt TEXT           Structured caption (or use --prompt-file)

OPTIONS:
    --prompt-file PATH      Read caption from a UTF-8 text file
    --lyrics TEXT           Lyrics with [Verse]/[Chorus] tags (empty = instrumental)
    --lyrics-file PATH      Read lyrics from file (mutually exclusive with --lyrics)
    --duration F            Target length in seconds, clamped to 1..360 [default: 60]
    --seed N                [default: 0]
    --max-new-tokens N      Acoustic frame budget override (25 fps, hard cap 9000)
    --output PATH           Destination .wav
"#;

pub const VIDEO_HELP: &str = r#"comfy video — Bernini-R text-to-video / video-to-video (MP4)

USAGE:
    comfy [GLOBAL OPTIONS] video --model PATH --prompt TEXT [OPTIONS]

REQUIRED:
    --model PATH            Bernini-R self-contained model directory
    --prompt TEXT           (or use --prompt-file)

OPTIONS:
    --prompt-file PATH
    --negative-prompt TEXT
    --width N               t2v only [default: 832]
    --height N              t2v only [default: 480]
    --frames N              [default: 81]
    --fps N                 [default: 16]
    --steps N               [default: 40]
    --seed N                [default: 42]
    --max-image-size N      Long-edge resize budget [default: 848]
    --input-video PATH      Source clip; switches the task to v2v
    --guidance-mode MODE    auto | t2v_apg | v2v_apg [default: auto]
    --output PATH           Destination .mp4
"#;

pub const AV_HELP: &str = r#"comfy av — MiniMax-H3 joint audio-video generation (MP4 with stereo track)

USAGE:
    comfy [GLOBAL OPTIONS] av --model PATH --prompt TEXT [OPTIONS]

REQUIRED:
    --model PATH            MiniMax-H3 modular repository
    --prompt TEXT           (or use --prompt-file)

OPTIONS:
    --prompt-file PATH
    --mode MODE             t2va | i2va | ref2va | mr2va [default: t2va]
    --negative-prompt TEXT
    --width N               [default: 832]
    --height N              [default: 480]
    --frames N              Aligned to 17n+5 (123..362) [default: 124]
    --fps N                 [default: 24]
    --steps N               [default: 50]
    --cfg F                 Guidance scale [default: 7.0]
    --seed N                [default: 42]
    --reference PATH        Reference image (repeatable; required for non-t2va)
    --sfx / --no-sfx        Generate sound effects [default: on]
    --bgm / --no-bgm        Generate background music [default: off]
    --variant V             fl2va | ref2va (normally auto-selected from --mode)
    --output PATH           Destination .mp4 (requires ffmpeg)
"#;

pub const GAUSSIAN3D_HELP: &str = r#"comfy gaussian3d — TripoSplat image-to-3D Gaussian splats

USAGE:
    comfy [GLOBAL OPTIONS] gaussian3d --ckpt PATH --decoder PATH --dinov3 PATH \
        --vae-encoder PATH --rmbg PATH --input-image PATH [OPTIONS]

REQUIRED MODEL PATHS:
    --ckpt PATH             TripoSplat checkpoint
    --decoder PATH          Gaussian decoder (triposplat_vae_decoder)
    --dinov3 PATH           DINOv3 clip-vision weights
    --vae-encoder PATH      FLUX2 VAE encoder
    --rmbg PATH             BiRefNet background-removal weights
    --input-image PATH      Source image

OPTIONS:
    --steps N               [default: 20]
    --guidance-scale F      [default: 3.0]
    --seed N                [default: 42]
    --num-gaussians N       [default: 262144]
    --erode-radius N        [default: 1]
    --format FMT            ply | splat [default: ply]
    --output PATH           Destination file
"#;

pub const CAPABILITIES_HELP: &str = r#"comfy capabilities — report resolved interpreter, script directory and the
backend's per-modality support flags as JSON. Loads no models.

USAGE:
    comfy [GLOBAL OPTIONS] capabilities
"#;

pub fn command_help(command: &str) -> Option<&'static str> {
    match command {
        "image" => Some(IMAGE_HELP),
        "music" => Some(MUSIC_HELP),
        "video" => Some(VIDEO_HELP),
        "av" => Some(AV_HELP),
        "gaussian3d" => Some(GAUSSIAN3D_HELP),
        "capabilities" => Some(CAPABILITIES_HELP),
        _ => None,
    }
}

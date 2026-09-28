//! Local LLM execution: llama-cli first, Python HF fallback second.
//!
//! Both the `LLMTextGen` and `AudioToLLM` builtin nodes share this runner.
//! The primary path shells out to `llama-cli` (gguf models); when the model
//! directory is a HuggingFace checkpoint, or llama-cli fails and the fallback
//! is enabled, generation is retried through
//! `py/flash_attn_v100/comfy_fallback/llm_generate.py` (transformers).

use comfy_inference::{is_hf_model_dir, resolve_python, resolve_script_dir};
use std::path::PathBuf;
use std::process::Stdio;

/// Machine-local fallback used only when `llm_config.cli_path` is missing.
const DEFAULT_LLAMA_CLI: &str =
    "/home/acproject/workspace/rust_projects/comfyui-rust/cpp/llama.cpp/build/bin/llama-cli";

/// Parameters for one local LLM generation call.
pub struct LlmRunRequest<'a> {
    pub model_path: &'a str,
    pub prompt: &'a str,
    pub system: Option<&'a str>,
    pub audio: Option<&'a str>,
    pub mmproj: Option<&'a str>,
    pub max_tokens: i64,
    pub temperature: f64,
    pub top_p: Option<f64>,
    pub seed: Option<i64>,
}

fn config_str<'a>(cfg: &'a serde_json::Value, key: &str) -> Option<&'a str> {
    cfg.get(key).and_then(|v| v.as_str()).filter(|s| !s.is_empty())
}

fn config_bool(cfg: &serde_json::Value, key: &str, default: bool) -> bool {
    cfg.get(key).and_then(|v| v.as_bool()).unwrap_or(default)
}

/// Resolve the llama-cli binary: config -> cwd-relative build dir -> known path.
fn resolve_cli_path(cfg: &serde_json::Value) -> String {
    if let Some(p) = config_str(cfg, "cli_path") {
        return p.to_string();
    }
    if let Ok(p) = std::env::var("LLAMA_CLI_PATH") {
        if !p.is_empty() {
            return p;
        }
    }
    let cwd_relative = PathBuf::from("cpp/llama.cpp/build/bin/llama-cli");
    if cwd_relative.exists() {
        return cwd_relative.to_string_lossy().to_string();
    }
    DEFAULT_LLAMA_CLI.to_string()
}

async fn run_llama_cli(cfg: &serde_json::Value, req: &LlmRunRequest<'_>) -> Result<String, String> {
    let cli_path = resolve_cli_path(cfg);

    let mut cmd = tokio::process::Command::new(&cli_path);
    cmd.arg("-m").arg(req.model_path)
        .arg("-p").arg(req.prompt)
        .arg("--n-predict").arg(req.max_tokens.to_string())
        .arg("--temp").arg(req.temperature.to_string())
        .arg("--no-display-prompt")
        .arg("--log-disable")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    if let Some(top_p) = req.top_p {
        cmd.arg("--top-p").arg(top_p.to_string());
    }
    if let Some(audio) = req.audio {
        if !audio.is_empty() {
            cmd.arg("--audio").arg(audio);
        }
    }
    if let Some(mmproj) = req.mmproj {
        if !mmproj.is_empty() {
            cmd.arg("--mmproj").arg(mmproj);
        }
    }
    if let Some(seed) = req.seed {
        if seed >= 0 {
            cmd.arg("--seed").arg(seed.to_string());
        }
    }
    if let Some(system) = req.system {
        if !system.is_empty() {
            cmd.arg("--system-prompt").arg(system);
        }
    }
    if let Some(extra_args) = config_str(cfg, "extra_args") {
        for arg in extra_args.split_whitespace() {
            cmd.arg(arg);
        }
    }

    match cmd.output().await {
        Ok(output) if output.status.success() => {
            Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
        }
        Ok(output) => {
            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
            Err(format!(
                "llama-cli ({}) exited with {}: {}",
                cli_path,
                output.status,
                stderr.lines().rev().take(15).collect::<Vec<_>>().into_iter().rev().collect::<Vec<_>>().join("\n")
            ))
        }
        Err(e) => Err(format!("Failed to execute llama-cli ({}): {}", cli_path, e)),
    }
}

/// Whether the Python fallback scripts + interpreter are available.
fn python_script(cfg: &serde_json::Value) -> Option<(String, PathBuf)> {
    let explicit = config_str(cfg, "python_script_dir");
    let script_dir = resolve_script_dir(explicit, None)?;
    let script = script_dir.join("llm_generate.py");
    if !script.exists() {
        return None;
    }
    let interpreter = resolve_python(config_str(cfg, "python_path"), Some(&script_dir));
    Some((interpreter, script))
}

async fn run_python_fallback(cfg: &serde_json::Value, req: &LlmRunRequest<'_>) -> Result<String, String> {
    let (interpreter, script) = python_script(cfg).ok_or_else(|| {
        "Python fallback unavailable: py/flash_attn_v100/comfy_fallback/llm_generate.py not found"
            .to_string()
    })?;

    let mut cmd = tokio::process::Command::new(&interpreter);
    cmd.arg(&script)
        .arg("--model-path").arg(req.model_path)
        .arg("--prompt").arg(req.prompt)
        .arg("--max-new-tokens").arg(req.max_tokens.to_string())
        .arg("--temperature").arg(req.temperature.to_string())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    if let Some(top_p) = req.top_p {
        cmd.arg("--top-p").arg(top_p.to_string());
    }
    if let Some(audio) = req.audio {
        if !audio.is_empty() {
            cmd.arg("--audio").arg(audio);
        }
    }
    if let Some(mmproj) = req.mmproj {
        if !mmproj.is_empty() {
            cmd.arg("--mmproj").arg(mmproj);
        }
    }
    if let Some(seed) = req.seed {
        if seed >= 0 {
            cmd.arg("--seed").arg(seed.to_string());
        }
    }
    if let Some(system) = req.system {
        if !system.is_empty() {
            cmd.arg("--system").arg(system);
        }
    }
    if let Some(dtype) = config_str(cfg, "python_dtype") {
        if dtype != "auto" {
            cmd.arg("--dtype").arg(dtype);
        }
    }

    tracing::info!(
        "Python HF LLM fallback: {} {}",
        interpreter,
        script.display()
    );

    match cmd.output().await {
        Ok(output) if output.status.success() => {
            Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
        }
        Ok(output) => {
            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
            Err(format!(
                "llm_generate.py exited with {}: {}",
                output.status,
                stderr.lines().rev().take(15).collect::<Vec<_>>().into_iter().rev().collect::<Vec<_>>().join("\n")
            ))
        }
        Err(e) => Err(format!("Failed to execute Python fallback ({}): {}", interpreter, e)),
    }
}

/// Run a local LLM generation according to `llm_config`.
///
/// Modes:
/// - `"remote"`: not handled here (nodes do HTTP themselves).
/// - `"python"`: always use the HF Python fallback.
/// - otherwise (`"local"`): llama-cli first; HF checkpoint directories go
///   straight to Python; llama-cli failures are retried with Python when
///   `python_fallback` is enabled (default true).
pub async fn run_local_or_python(
    cfg: &serde_json::Value,
    request: LlmRunRequest<'_>,
) -> Result<String, String> {
    let mode = config_str(cfg, "mode").unwrap_or("local");
    if mode == "remote" {
        return Err("remote LLM mode must use the HTTP completions path".to_string());
    }
    if request.model_path.is_empty() {
        return Err("LLM model path is empty".to_string());
    }
    if mode == "python" {
        return run_python_fallback(cfg, &request).await;
    }

    let fallback_enabled = config_bool(cfg, "python_fallback", true);
    let hf_checkpoint = is_hf_model_dir(request.model_path);

    if fallback_enabled && hf_checkpoint && python_script(cfg).is_some() {
        tracing::info!(
            "Model '{}' is a HuggingFace checkpoint directory; using Python fallback directly",
            request.model_path
        );
        return run_python_fallback(cfg, &request).await;
    }

    match run_llama_cli(cfg, &request).await {
        Ok(text) => Ok(text),
        Err(cli_err) => {
            if !fallback_enabled || python_script(cfg).is_none() {
                return Err(cli_err);
            }
            tracing::warn!(
                "llama-cli failed ({}); retrying with Python HF fallback",
                cli_err
            );
            match run_python_fallback(cfg, &request).await {
                Ok(text) => Ok(text),
                Err(py_err) => Err(format!(
                    "llama-cli error: {} | Python fallback error: {}",
                    cli_err, py_err
                )),
            }
        }
    }
}

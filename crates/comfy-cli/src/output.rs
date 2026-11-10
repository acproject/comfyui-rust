//! Machine-readable output contract.
//!
//! Every successful invocation prints exactly one JSON object on stdout:
//! `{"ok":true,"command":...,"outputs":[...],"meta":{...}}`; failures print
//! `{"ok":false,"error":...}`. All human logs stay on stderr.

use std::fs;
use std::path::{Path, PathBuf};

use comfy_inference::InferenceError;
use serde::Serialize;
use serde_json::Value;

#[derive(Debug, Clone, Serialize)]
pub struct OutputEntry {
    pub path: String,
    pub media_type: String,
    pub format: String,
}

#[derive(Debug, Serialize)]
struct Envelope {
    ok: bool,
    command: String,
    outputs: Vec<OutputEntry>,
    meta: Value,
}

pub fn entry(path: impl Into<String>, media_type: &str, format: &str) -> OutputEntry {
    OutputEntry {
        path: path.into(),
        media_type: media_type.to_string(),
        format: format.to_string(),
    }
}

pub fn print_success(command: &str, outputs: Vec<OutputEntry>, meta: Value) {
    let envelope = Envelope {
        ok: true,
        command: command.to_string(),
        outputs,
        meta,
    };
    match serde_json::to_string(&envelope) {
        Ok(line) => println!("{line}"),
        Err(_) => println!("{}", serde_json::json!({"ok": true})),
    }
}

pub fn print_failure(error: &str) {
    println!("{}", serde_json::json!({"ok": false, "error": error}));
}

/// Resolve the destination file: explicit `--output` wins, otherwise
/// `<output-dir>/<name>.<ext>`. Parent directories are created.
pub fn prepare_path(
    output: &Option<String>,
    output_dir: &str,
    name: &str,
    ext: &str,
) -> Result<PathBuf, InferenceError> {
    let path = match output {
        Some(raw) => PathBuf::from(raw),
        None => PathBuf::from(output_dir).join(format!("{name}.{ext}")),
    };
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent)?;
        }
    }
    Ok(path)
}

/// Insert `_<index>` before the extension for multi-output commands.
pub fn indexed_path(base: &Path, index: usize) -> PathBuf {
    let stem = base
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "output".to_string());
    let ext = base
        .extension()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_default();
    let file = if ext.is_empty() {
        format!("{stem}_{}", index + 1)
    } else {
        format!("{stem}_{}.{ext}", index + 1)
    };
    base.with_file_name(file)
}

/// Absolute, normalized path string for the result envelope.
pub fn abs_string(path: &Path) -> String {
    if path.is_absolute() {
        return path.to_string_lossy().to_string();
    }
    std::env::current_dir()
        .map(|cwd| cwd.join(path))
        .unwrap_or_else(|_| path.to_path_buf())
        .to_string_lossy()
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_path_uses_output_dir_and_creates_parents() {
        let dir = std::env::temp_dir().join("comfy_cli_output_test");
        let dir_str = dir.to_string_lossy().to_string();
        let path = prepare_path(&None, &dir_str, "image_42", "png").unwrap();
        assert_eq!(path, dir.join("image_42.png"));
        assert!(dir.is_dir(), "prepare_path must create the output directory");
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn explicit_output_path_wins_and_keeps_extension() {
        let path =
            prepare_path(&Some("/tmp/comfy_explicit/x.wav".to_string()), "/ignored", "music_1", "wav")
                .unwrap();
        assert_eq!(path, PathBuf::from("/tmp/comfy_explicit/x.wav"));
        assert!(Path::new("/tmp/comfy_explicit").is_dir());
        fs::remove_dir_all("/tmp/comfy_explicit").ok();
    }

    #[test]
    fn indexed_path_inserts_counter_before_extension() {
        let base = PathBuf::from("/tmp/out/image_1.png");
        assert_eq!(indexed_path(&base, 0), PathBuf::from("/tmp/out/image_1_1.png"));
        assert_eq!(indexed_path(&base, 2), PathBuf::from("/tmp/out/image_1_3.png"));
        let no_ext = PathBuf::from("/tmp/out/blob");
        assert_eq!(indexed_path(&no_ext, 0), PathBuf::from("/tmp/out/blob_1"));
    }

    #[test]
    fn abs_string_passthrough_and_relative_join() {
        assert_eq!(abs_string(Path::new("/abs/x.png")), "/abs/x.png");
        let joined = abs_string(Path::new("rel/x.png"));
        assert!(joined.ends_with("rel/x.png"));
        assert!(Path::new(&joined).is_absolute());
    }
}

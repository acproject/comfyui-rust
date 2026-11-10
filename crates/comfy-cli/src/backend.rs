//! Inference backend construction for the standalone CLI.
//!
//! The CLI talks to backends in-process (no comfy-server required): a
//! [`PythonBackend`] drives the `comfy_fallback` scripts directly, and an
//! optional `--sd-cli` wraps it inside a [`FallbackBackend`] so sd.cpp-native
//! models keep their primary path while HF/diffusers models fall through.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use comfy_inference::{
    CliBackend, CliBackendConfig, FallbackBackend, InferenceBackend, PythonBackend,
    PythonInferConfig, resolve_script_dir,
};

use crate::args::GlobalOpts;
use comfy_inference::InferenceError;

/// Workspace root inferred from this crate's manifest location at compile time:
/// `<root>/crates/comfy-cli` -> two ancestors up.
pub fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .map(PathBuf::from)
        .expect("comfy-cli manifest always lives under <root>/crates/comfy-cli")
}

fn python_config(g: &GlobalOpts) -> PythonInferConfig {
    PythonInferConfig {
        python_path: g.python.clone(),
        script_dir: g.script_dir.clone(),
        device: g.device.clone(),
        dtype: g.dtype.clone(),
        enabled: true,
    }
}

/// Construct the Python backend directly (also used for lightweight capability
/// introspection; construction only resolves paths and spawns nothing).
pub fn build_python(g: &GlobalOpts) -> PythonBackend {
    PythonBackend::new(python_config(g), Some(&workspace_root()))
}

/// Build the trait object used by every generation subcommand.
pub fn build(g: &GlobalOpts) -> Result<Arc<dyn InferenceBackend>, InferenceError> {
    let python = build_python(g);
    let backend: Arc<dyn InferenceBackend> = match &g.sd_cli {
        Some(sd_cli_path) => {
            let cli = CliBackend::new(CliBackendConfig::new(sd_cli_path.clone()))?;
            Arc::new(FallbackBackend::new(Arc::new(cli), python))
        }
        None => Arc::new(python),
    };
    Ok(backend)
}

/// The fallback script directory after all resolution stages
/// (explicit flag -> COMFY_PYTHON_SCRIPT_DIR -> workspace -> cwd).
pub fn resolved_script_dir(g: &GlobalOpts) -> Option<PathBuf> {
    resolve_script_dir(g.script_dir.as_deref(), Some(&workspace_root()))
}

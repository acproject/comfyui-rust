#![allow(non_camel_case_types)]

pub mod backend;
pub mod cli;
pub mod error;
pub mod image;
pub mod params;
pub mod python;
pub mod sdcpp_support;
pub mod types;

#[cfg(feature = "local-ffi")]
pub mod ffi;
#[cfg(feature = "local-ffi")]
pub mod local;

#[cfg(feature = "remote")]
pub mod remote;

#[cfg(feature = "flash-attn")]
pub mod flash_attn_backend;

#[cfg(any(windows, feature = "comfyui-bridge"))]
pub mod windows_backend;

/// Cross-platform blocking helper shared by the HTTP-backed inference backends
/// (`flash_attn_backend` on all platforms, `windows_backend` on Windows).
///
/// On Windows (or when `comfyui-bridge` is enabled on Unix) this re-exports
/// [`windows_backend::run_blocking`]. Everywhere else it is the standalone
/// [`cross_platform_blocking`] module, so both backends compile and call one
/// helper without cfg-gating each call site.
#[cfg(not(any(windows, feature = "comfyui-bridge")))]
pub mod cross_platform_blocking;
#[cfg(any(windows, feature = "comfyui-bridge"))]
pub mod cross_platform_blocking {
    pub use super::windows_backend::run_blocking;
}

pub use backend::{AsyncInferenceBackend, BackendCapabilities, InferenceBackend, NullBackend};
pub use cli::{CliBackend, CliBackendConfig, convert_model_cli};
pub use python::{
    FallbackBackend, PythonBackend, PythonInferConfig, default_script_dir, is_diffusers_pipeline_dir,
    is_hf_model_dir, resolve_python, resolve_script_dir, venv_python_path,
};
pub use sdcpp_support::{
    RegistrySnapshot, SdCppModelEntry, SdCppModelKind, UPSTREAM_README, init as sdcpp_init,
    match_identifier as sdcpp_match, merge_upstream_readme, parse_upstream_readme,
    snapshot as sdcpp_snapshot,
};
pub use error::{GenerationMode, InferenceError, InferenceResult};
pub use image::{ImageError, SdImage, SdVideo, SdAudio};
pub use params::*;
pub use types::*;

#[cfg(feature = "local-ffi")]
pub use local::{LocalBackend, convert_model, get_system_info, get_version, get_commit, get_num_physical_cores};

#[cfg(feature = "remote")]
pub use remote::{RemoteBackend, RemoteConfig};

#[cfg(feature = "flash-attn")]
pub use flash_attn_backend::{FlashAttnBackend, FlashAttnConfig, FlashProgressCallback, HealthResponse, JobResponse};

#[cfg(any(windows, feature = "comfyui-bridge"))]
pub use windows_backend::{WindowsBackend, WindowsBackendConfig, DEFAULT_COMFY_ROOT, DEFAULT_COMFY_URL};

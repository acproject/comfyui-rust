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

pub use backend::{AsyncInferenceBackend, BackendCapabilities, InferenceBackend, NullBackend};
pub use cli::{CliBackend, CliBackendConfig, convert_model_cli};
pub use python::{
    FallbackBackend, PythonBackend, PythonInferConfig, default_script_dir, is_diffusers_pipeline_dir,
    is_hf_model_dir, resolve_python, resolve_script_dir,
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

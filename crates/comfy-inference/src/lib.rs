#![allow(non_camel_case_types)]

pub mod backend;
pub mod cli;
pub mod error;
pub mod image;
pub mod params;
pub mod types;

#[cfg(feature = "local-ffi")]
pub mod ffi;
#[cfg(feature = "local-ffi")]
pub mod local;

#[cfg(feature = "remote")]
pub mod remote;

pub use backend::{AsyncInferenceBackend, BackendCapabilities, InferenceBackend, NullBackend};
pub use cli::{CliBackend, CliBackendConfig, convert_model_cli};
pub use error::{GenerationMode, InferenceError, InferenceResult};
pub use image::{ImageError, SdImage, SdVideo};
pub use params::*;
pub use types::*;

#[cfg(feature = "local-ffi")]
pub use local::{LocalBackend, convert_model, get_system_info, get_version, get_commit, get_num_physical_cores};

#[cfg(feature = "remote")]
pub use remote::{RemoteBackend, RemoteConfig};

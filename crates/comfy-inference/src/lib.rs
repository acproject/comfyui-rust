#![allow(non_camel_case_types)]

pub mod backend;
pub mod error;
pub mod image;
pub mod params;
pub mod types;

#[cfg(feature = "local")]
pub mod ffi;
#[cfg(feature = "local")]
pub mod local;

#[cfg(feature = "remote")]
pub mod remote;

pub use backend::{AsyncInferenceBackend, BackendCapabilities, InferenceBackend, NullBackend};
pub use error::{GenerationMode, InferenceError, InferenceResult};
pub use image::{ImageError, SdImage, SdVideo};
pub use params::*;
pub use types::*;

#[cfg(feature = "local")]
pub use local::LocalBackend;

#[cfg(feature = "remote")]
pub use remote::{RemoteBackend, RemoteConfig};

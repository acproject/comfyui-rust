use std::fmt;

#[derive(Debug, thiserror::Error)]
pub enum InferenceError {
    #[error("Context creation failed")]
    ContextCreationFailed,

    #[error("Model not loaded: {0}")]
    ModelNotLoaded(String),

    #[error("Generation failed: {0}")]
    GenerationFailed(String),

    #[error("Invalid parameter: {0}")]
    InvalidParameter(String),

    #[error("FFI error: {0}")]
    FfiError(String),

    #[error("Image decode error: {0}")]
    ImageDecodeError(String),

    #[error("Image encode error: {0}")]
    ImageEncodeError(String),

    #[error("Remote server error: {status} - {message}")]
    RemoteError { status: u16, message: String },

    #[error("Network error: {0}")]
    NetworkError(String),

    #[error("Unsupported operation: {0}")]
    UnsupportedOperation(String),

    #[error("Backend not available: {0}")]
    BackendNotAvailable(String),
}

pub type InferenceResult<T> = Result<T, InferenceError>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GenerationMode {
    ImageGeneration,
    VideoGeneration,
}

impl fmt::Display for GenerationMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GenerationMode::ImageGeneration => write!(f, "img_gen"),
            GenerationMode::VideoGeneration => write!(f, "vid_gen"),
        }
    }
}

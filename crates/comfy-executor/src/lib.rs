pub mod builtin_nodes;
pub mod error;
pub mod execution_context;
pub mod executor;
pub mod llm_runner;
pub mod registry;

#[cfg(feature = "controlnet")]
pub mod controlnet;

pub mod bernini;
pub mod minimax;
pub mod mask;
pub mod prompt_relay;
pub mod triposplat;

/// Serializes unit tests that mutate process-global state (`COMFY_MODELS_DIR`,
/// current working directory) so parallel test threads do not race.
#[cfg(test)]
pub static TEST_ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

pub use error::{ExecutorError, ErrorDetail, NodeErrorInfo, ValidationResult};
pub use execution_context::{ExecutionContext, NodeOutput, ProgressCallback};
pub use executor::{Executor, ExecutionResult, NodeEventCallback};
pub use registry::NodeRegistry;

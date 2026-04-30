use comfy_core::DependencyCycleError;
use std::collections::HashMap;

#[derive(Debug, thiserror::Error)]
pub enum ExecutorError {
    #[error("Node '{node_id}' not found in prompt")]
    NodeNotFound { node_id: String },

    #[error("Node type '{class_type}' not registered")]
    NodeTypeNotRegistered { class_type: String },

    #[error("Missing required input '{input}' on node '{node_id}'")]
    MissingInput { node_id: String, input: String },

    #[error("Output index {index} out of bounds on node '{node_id}' (max {max})")]
    OutputIndexOutOfBounds { node_id: String, index: usize, max: usize },

    #[error("Execution failed for node '{node_id}': {message}")]
    NodeExecutionFailed { node_id: String, message: String },

    #[error("Dependency cycle detected")]
    DependencyCycle(#[from] DependencyCycleError),

    #[error("Validation failed: {0}")]
    ValidationFailed(String),

    #[error("Inference error: {0}")]
    Inference(#[from] comfy_inference::InferenceError),

    #[error("Prompt has no output nodes")]
    NoOutputNodes,

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct NodeErrorInfo {
    pub node_id: String,
    pub class_type: String,
    pub errors: Vec<ErrorDetail>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ErrorDetail {
    pub error_type: String,
    pub message: String,
    pub details: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ValidationResult {
    pub valid: bool,
    pub error: Option<ErrorDetail>,
    pub node_errors: HashMap<String, NodeErrorInfo>,
}

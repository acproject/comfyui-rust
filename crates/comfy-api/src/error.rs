use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde_json::json;

#[derive(Debug, thiserror::Error)]
pub enum ApiError {
    #[error("Bad request: {0}")]
    BadRequest(String),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Validation failed")]
    Validation(comfy_executor::ValidationResult),

    #[error("Execution error: {0}")]
    Execution(#[from] comfy_executor::ExecutorError),

    #[error("Inference error: {0}")]
    Inference(#[from] comfy_inference::InferenceError),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Internal error: {0}")]
    Internal(String),
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, error_json) = match &self {
            ApiError::BadRequest(msg) => (
                StatusCode::BAD_REQUEST,
                json!({
                    "error": {
                        "type": "bad_request",
                        "message": msg,
                        "details": msg,
                    },
                    "node_errors": {}
                }),
            ),
            ApiError::NotFound(msg) => (
                StatusCode::NOT_FOUND,
                json!({
                    "error": {
                        "type": "not_found",
                        "message": msg,
                        "details": msg,
                    }
                }),
            ),
            ApiError::Validation(result) => (
                StatusCode::BAD_REQUEST,
                json!({
                    "error": result.error,
                    "node_errors": result.node_errors,
                }),
            ),
            ApiError::Execution(e) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                json!({
                    "error": {
                        "type": "execution_error",
                        "message": e.to_string(),
                        "details": format!("{:?}", e),
                    },
                    "node_errors": {}
                }),
            ),
            ApiError::Inference(e) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                json!({
                    "error": {
                        "type": "inference_error",
                        "message": e.to_string(),
                        "details": format!("{:?}", e),
                    }
                }),
            ),
            ApiError::Json(e) => (
                StatusCode::BAD_REQUEST,
                json!({
                    "error": {
                        "type": "json_error",
                        "message": e.to_string(),
                        "details": format!("{:?}", e),
                    }
                }),
            ),
            ApiError::Internal(msg) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                json!({
                    "error": {
                        "type": "internal_error",
                        "message": msg,
                        "details": msg,
                    }
                }),
            ),
        };

        (status, axum::Json(error_json)).into_response()
    }
}

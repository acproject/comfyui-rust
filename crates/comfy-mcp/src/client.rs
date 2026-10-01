//! Thin REST client for a running comfy-server.
//!
//! Every MCP tool operates against the real server so node schemas, model
//! listings and queue state always match what the user sees in the UI.
//! The base URL is configurable (env `COMFY_SERVER_URL`, default
//! `http://127.0.0.1:8188`).

use serde_json::Value;
use std::time::Duration;

/// Default comfy-server endpoint.
pub const DEFAULT_SERVER_URL: &str = "http://127.0.0.1:8188";

/// Resolve the comfy-server base URL from `COMFY_SERVER_URL`.
pub fn default_server_url() -> String {
    std::env::var("COMFY_SERVER_URL").unwrap_or_else(|_| DEFAULT_SERVER_URL.to_string())
}

/// Errors returned by [`ComfyClient`].
#[derive(Debug, thiserror::Error)]
pub enum ClientError {
    /// Transport-level failure (server not reachable, connection reset, ...).
    #[error("cannot reach comfy-server at {url}: {source}")]
    Connect { url: String, source: reqwest::Error },

    /// Request reached the server but produced a non-success status.
    #[error("comfy-server returned HTTP {status} for {method} {url}: {body}")]
    Http {
        method: String,
        url: String,
        status: u16,
        body: String,
    },

    /// Response body could not be decoded as JSON.
    #[error("invalid JSON from comfy-server ({url}): {source}")]
    Decode { url: String, source: reqwest::Error },
}

/// REST client for comfy-server (ComfyUI-compatible endpoints, root paths).
#[derive(Clone)]
pub struct ComfyClient {
    http: reqwest::Client,
    base_url: String,
}

impl std::fmt::Debug for ComfyClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ComfyClient")
            .field("base_url", &self.base_url)
            .finish()
    }
}

impl ComfyClient {
    /// Build a client pointing at `base_url` (trailing slash trimmed).
    pub fn new(base_url: impl Into<String>) -> Self {
        let base_url = base_url.into().trim_end_matches('/').to_string();
        let http = reqwest::Client::builder()
            .timeout(Duration::from_secs(60))
            .build()
            .expect("failed to build reqwest client");
        Self { http, base_url }
    }

    /// Build a client using [`default_server_url`].
    pub fn from_env() -> Self {
        Self::new(default_server_url())
    }

    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    fn url(&self, path: &str) -> String {
        format!("{}{}", self.base_url, path)
    }

    async fn get_json(&self, path: &str) -> Result<Value, ClientError> {
        let url = self.url(path);
        let resp = self
            .http
            .get(&url)
            .send()
            .await
            .map_err(|source| ClientError::Connect {
                url: url.clone(),
                source,
            })?;
        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(ClientError::Http {
                method: "GET".to_string(),
                url,
                status: status.as_u16(),
                body: truncate_body(&body),
            });
        }
        resp.json::<Value>().await.map_err(|source| ClientError::Decode {
            url,
            source,
        })
    }

    async fn post_json(&self, path: &str, body: Value) -> Result<Value, ClientError> {
        let url = self.url(path);
        let resp = self
            .http
            .post(&url)
            .json(&body)
            .send()
            .await
            .map_err(|source| ClientError::Connect {
                url: url.clone(),
                source,
            })?;
        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(ClientError::Http {
                method: "POST".to_string(),
                url,
                status: status.as_u16(),
                body: truncate_body(&body),
            });
        }
        resp.json::<Value>().await.map_err(|source| ClientError::Decode {
            url,
            source,
        })
    }

    async fn post_empty(&self, path: &str) -> Result<(), ClientError> {
        let url = self.url(path);
        let resp = self
            .http
            .post(&url)
            .send()
            .await
            .map_err(|source| ClientError::Connect {
                url: url.clone(),
                source,
            })?;
        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(ClientError::Http {
                method: "POST".to_string(),
                url,
                status: status.as_u16(),
                body: truncate_body(&body),
            });
        }
        Ok(())
    }

    /// Connectivity + basic server information.
    pub async fn ping(&self) -> Result<Value, ClientError> {
        self.get_json("/system_stats").await
    }

    /// Full `/object_info` registry: class_type -> schema object.
    pub async fn object_info(&self) -> Result<Value, ClientError> {
        self.get_json("/object_info").await
    }

    /// Schema for a single node class.
    pub async fn object_info_class(&self, class_type: &str) -> Result<Value, ClientError> {
        let path = format!(
            "/object_info/{}",
            urlencoding_path_segment(class_type)
        );
        self.get_json(&path).await
    }

    /// Local model files, keyed by model type folder.
    /// `model_type = None` lists every folder.
    pub async fn list_models(&self, model_type: Option<&str>) -> Result<Value, ClientError> {
        let path = match model_type {
            Some(mt) => format!("/model_manager/list?model_type={}", urlencoding_path_segment(mt)),
            None => "/model_manager/list".to_string(),
        };
        self.get_json(&path).await
    }

    /// Built-in workflow templates.
    pub async fn list_templates(&self) -> Result<Value, ClientError> {
        self.get_json("/workflow_templates").await
    }

    /// A single workflow template (nodes + connections).
    pub async fn get_template(&self, template_id: &str) -> Result<Value, ClientError> {
        let path = format!(
            "/workflow_templates/{}",
            urlencoding_path_segment(template_id)
        );
        self.get_json(&path).await
    }

    /// Submit a prompt graph for execution. Returns the server response
    /// (`prompt_id`, `number`, `node_errors`).
    pub async fn submit_prompt(
        &self,
        prompt: Value,
        client_id: Option<&str>,
    ) -> Result<Value, ClientError> {
        let mut body = serde_json::json!({
            "prompt": prompt,
            "extra_data": {},
        });
        if let Some(cid) = client_id {
            body["client_id"] = Value::String(cid.to_string());
        }
        self.post_json("/prompt", body).await
    }

    pub async fn queue(&self) -> Result<Value, ClientError> {
        self.get_json("/queue").await
    }

    pub async fn history(&self, max_items: Option<usize>) -> Result<Value, ClientError> {
        let path = match max_items {
            Some(n) => format!("/history?max_items={}", n),
            None => "/history".to_string(),
        };
        self.get_json(&path).await
    }

    pub async fn history_by_id(&self, prompt_id: &str) -> Result<Value, ClientError> {
        let path = format!("/history/{}", urlencoding_path_segment(prompt_id));
        self.get_json(&path).await
    }

    pub async fn interrupt(&self) -> Result<(), ClientError> {
        self.post_empty("/interrupt").await
    }
}

fn truncate_body(body: &str) -> String {
    const MAX: usize = 800;
    if body.len() <= MAX {
        body.to_string()
    } else {
        format!("{}... (truncated)", &body[..MAX])
    }
}

/// Percent-encode a single path/query segment (class types may contain spaces
/// or other characters in custom nodes).
fn urlencoding_path_segment(segment: &str) -> String {
    const UNRESERVED: &[u8] = b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789-_.~";
    let mut out = String::with_capacity(segment.len());
    for b in segment.bytes() {
        if UNRESERVED.contains(&b) {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{:02X}", b));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trims_trailing_slash_and_encodes_segments() {
        let c = ComfyClient::new("http://127.0.0.1:8188/");
        assert_eq!(c.base_url(), "http://127.0.0.1:8188");
        assert_eq!(urlencoding_path_segment("KSampler"), "KSampler");
        assert_eq!(urlencoding_path_segment("My Node v2"), "My%20Node%20v2");
    }
}

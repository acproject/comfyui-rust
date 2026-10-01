//! MCP (Model Context Protocol) Streamable HTTP endpoint mounted at `/mcp`.
//!
//! AI IDEs (Trae, Cursor, Claude Code, ...) can point their MCP client at
//! `http://<server>:<port>/mcp` to build controllable generation workflows
//! against this very server through the comfy-mcp tools.
//!
//! Configuration:
//! - `COMFY_MCP_ENABLED` — `0`/`false` disables the endpoint (default enabled).
//! - `COMFY_SERVER_URL` — REST base URL the MCP tools call. Defaults to
//!   `http://127.0.0.1:<listen-port>` (loopback self-call).
//! - `COMFY_MCP_ALLOWED_HOSTS` — comma-separated accepted `Host` headers
//!   (DNS-rebinding protection; default loopback).

use axum::body::Body as AxumBody;
use axum::extract::State;
use axum::http::{Request, Response};
use axum::routing::any;
use axum::Router;
use comfy_mcp::{ComfyClient, ComfyMcpHttpService, build_server, mcp_http_service};
use std::convert::Infallible;
use tower::ServiceExt;

/// Environment switch for the `/mcp` endpoint.
pub const ENV_MCP_ENABLED: &str = "COMFY_MCP_ENABLED";

fn mcp_enabled() -> bool {
    match std::env::var(ENV_MCP_ENABLED) {
        Ok(raw) => !matches!(raw.trim().to_ascii_lowercase().as_str(), "0" | "false" | "no" | "off"),
        Err(_) => true,
    }
}

/// Build the `/mcp` router, or `None` when disabled by configuration.
///
/// `listen_port` is used to construct the default self-call REST URL when
/// `COMFY_SERVER_URL` is not set.
pub fn mcp_router(listen_port: u16) -> Option<Router> {
    if !mcp_enabled() {
        tracing::info!("MCP endpoint disabled ({ENV_MCP_ENABLED}=0/false)");
        return None;
    }

    let server_url = std::env::var("COMFY_SERVER_URL")
        .ok()
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| format!("http://127.0.0.1:{listen_port}"));

    let client = ComfyClient::new(&server_url);
    let handler = build_server(client);
    // Allowed hosts come from COMFY_MCP_ALLOWED_HOSTS (or loopback default).
    let service = mcp_http_service(handler, Vec::new());

    tracing::info!(
        "MCP Streamable HTTP endpoint mounted at /mcp (REST upstream: {}, allowed hosts: {:?})",
        server_url,
        comfy_mcp::default_allowed_hosts()
    );

    Some(
        Router::new()
            .route("/mcp", any(mcp_handler))
            .route("/mcp/", any(mcp_handler))
            .with_state(service),
    )
}

async fn mcp_handler(
    State(service): State<ComfyMcpHttpService>,
    req: Request<AxumBody>,
) -> Response<AxumBody> {
    // rmcp's service error is Infallible (HTTP errors are carried in the
    // response itself).
    service
        .oneshot(req)
        .await
        .unwrap_or_else(|never: Infallible| match never {})
        .map(AxumBody::new)
}

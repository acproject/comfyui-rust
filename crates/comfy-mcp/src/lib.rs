//! # comfy-mcp
//!
//! [Model Context Protocol](https://modelcontextprotocol.io/) server for
//! ComfyUI-Rust. It lets mainstream AI IDEs (Trae, Cursor, Claude Code, ...)
//! **build controllable generation workflows** against a running comfy-server:
//! discover nodes/models, assemble and statically validate a graph, submit it,
//! and track execution — all over the existing REST API.
//!
//! Two transports are provided:
//!
//! * **stdio** — standalone `comfy-mcp` binary launched by the IDE
//!   (`COMFY_SERVER_URL` selects the server, default `http://127.0.0.1:8188`).
//! * **Streamable HTTP** — [`mcp_http_service`] returns a tower service
//!   mounted by comfy-api at `/mcp`.

pub mod client;
pub mod handler;
pub mod workflow;

pub use client::{ComfyClient, DEFAULT_SERVER_URL, default_server_url};
pub use handler::ComfyMcpServer;

use std::sync::Arc;

/// Environment variable overriding the allowed `Host` headers for the HTTP
/// transport (comma-separated, e.g. "localhost,127.0.0.1,0.0.0.0").
pub const ENV_ALLOWED_HOSTS: &str = "COMFY_MCP_ALLOWED_HOSTS";

/// Build an MCP server handler connected to the given REST client.
pub fn build_server(client: ComfyClient) -> ComfyMcpServer {
    ComfyMcpServer::new(client)
}

/// Serve the MCP server over stdio until the transport closes.
pub async fn serve_stdio(server: ComfyMcpServer) -> Result<(), rmcp::RmcpError> {
    use rmcp::ServiceExt;
    use rmcp::transport::stdio;
    let service = server.serve(stdio()).await?;
    service.waiting().await?;
    Ok(())
}

/// The tower [`tower::Service`] type backing the Streamable HTTP transport.
pub type ComfyMcpHttpService = rmcp::transport::streamable_http_server::StreamableHttpService<
    ComfyMcpServer,
    rmcp::transport::streamable_http_server::session::local::LocalSessionManager,
>;

/// Construct the Streamable HTTP MCP service (mount at `/mcp`).
///
/// Each request gets a cheap clone of the handler (the REST client is shared
/// via `Arc`). When `allowed_hosts` is empty, hosts are taken from
/// `COMFY_MCP_ALLOWED_HOSTS` (or loopback by default). To accept any Host
/// header (LAN deployment behind a trusted network), use
/// [`mcp_http_service_allow_any_host`].
pub fn mcp_http_service(
    server: ComfyMcpServer,
    allowed_hosts: Vec<String>,
) -> ComfyMcpHttpService {
    use rmcp::transport::streamable_http_server::{
        StreamableHttpServerConfig, StreamableHttpService,
        session::local::LocalSessionManager,
    };

    let hosts = if allowed_hosts.is_empty() {
        default_allowed_hosts()
    } else {
        allowed_hosts
    };
    let config = StreamableHttpServerConfig::default().with_allowed_hosts(hosts);

    let factory_server = server.clone();
    StreamableHttpService::new(
        move || Ok(factory_server.clone()),
        Arc::new(LocalSessionManager::default()),
        config,
    )
}

/// Streamable HTTP service that accepts any `Host` header. Only use on trusted
/// private networks (disables the DNS-rebinding protection).
pub fn mcp_http_service_allow_any_host(server: ComfyMcpServer) -> ComfyMcpHttpService {
    use rmcp::transport::streamable_http_server::{
        StreamableHttpServerConfig, StreamableHttpService,
        session::local::LocalSessionManager,
    };
    let config = StreamableHttpServerConfig::default().disable_allowed_hosts();
    let factory_server = server.clone();
    StreamableHttpService::new(
        move || Ok(factory_server.clone()),
        Arc::new(LocalSessionManager::default()),
        config,
    )
}

/// Default allowed hosts: `COMFY_MCP_ALLOWED_HOSTS` or loopback.
pub fn default_allowed_hosts() -> Vec<String> {
    match std::env::var(ENV_ALLOWED_HOSTS) {
        Ok(raw) => raw
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect(),
        Err(_) => vec!["localhost".into(), "127.0.0.1".into(), "::1".into()],
    }
}

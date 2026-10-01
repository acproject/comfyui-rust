//! Standalone MCP server over stdio for AI IDEs.
//!
//! IDE configuration (stdio transport):
//! ```json
//! {
//!   "mcpServers": {
//!     "comfyui-rust": {
//!       "command": "/path/to/comfy-mcp",
//!       "env": { "COMFY_SERVER_URL": "http://127.0.0.1:8188" }
//!     }
//!   }
//! }
//! ```

use comfy_mcp::{ComfyClient, build_server, serve_stdio};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    if std::env::var("RUST_LOG").is_err() {
        std::env::set_var("RUST_LOG", "warn");
    }
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .with_writer(std::io::stderr)
        .with_target(false)
        .init();

    // Optional explicit server URL override.
    let server_url = match parse_server_url(std::env::args().skip(1)) {
        Ok(url) => url,
        Err(msg) => {
            eprintln!("{msg}");
            eprintln!("usage: comfy-mcp [--server-url URL]");
            std::process::exit(2);
        }
    };

    tracing::info!(server_url = %server_url, "starting comfy-mcp stdio server");
    let client = ComfyClient::new(server_url);
    let server = build_server(client);
    serve_stdio(server).await?;
    Ok(())
}

fn parse_server_url(args: impl Iterator<Item = String>) -> Result<String, String> {
    let mut iter = args.peekable();
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--server-url" | "--url" => {
                let value = iter
                    .next()
                    .ok_or_else(|| "missing value for --server-url".to_string())?;
                return Ok(value);
            }
            "-h" | "--help" => {
                return Err("comfy-mcp: MCP stdio server for ComfyUI-Rust".to_string())
            }
            other if other.starts_with("--server-url=") => {
                return Ok(other.trim_start_matches("--server-url=").to_string());
            }
            other => return Err(format!("unexpected argument: {other}")),
        }
    }
    Ok(comfy_mcp::default_server_url())
}

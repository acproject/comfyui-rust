//! WebSocket client for the comfyui-rust server.
//!
//! The server only publishes real-time generation progress and execution
//! lifecycle events over `ws://host:8188/ws?clientId=...` — the HTTP `/queue`
//! endpoint carries just `prompt_id` + `number`. This module connects once,
//! reconnects on drop, and forwards parsed events to the UI thread over an
//! mpsc channel so the queue screen can render real progress / stages / logs.

use std::sync::mpsc::Sender;
use std::time::Duration;

/// Events parsed from the server's WebSocket stream.
#[derive(Debug, Clone)]
pub enum WsEvent {
    Connected,
    Disconnected,
    /// `status` broadcast — carries the same shape as GET /queue.
    Status(serde_json::Value),
    Progress { prompt_id: String, value: f64, max: f64, node: String },
    ExecStart { prompt_id: String },
    Executing { prompt_id: String, node: Option<String> },
    ExecSuccess { prompt_id: String, output: serde_json::Value },
    ExecError { prompt_id: String, error: String },
}

/// Convert an http(s) base URL into the ws(s) endpoint for the live socket.
fn ws_url(base: &str, client_id: &str) -> String {
    let trimmed = base.trim_end_matches('/');
    let scheme_replaced = if let Some(rest) = trimmed.strip_prefix("https://") {
        format!("wss://{}", rest)
    } else if let Some(rest) = trimmed.strip_prefix("http://") {
        format!("ws://{}", rest)
    } else {
        format!("ws://{}", trimmed)
    };
    format!("{}/ws?clientId={}", scheme_replaced, client_id)
}

/// Spawn a background thread that keeps a WebSocket open to `server_url`,
/// forwarding parsed [`WsEvent`]s to `tx`. The thread reconnects forever on
/// disconnect; dropping the only receiver lets the thread exit on its next
/// send failure.
pub fn spawn(server_url: String, client_id: String, tx: Sender<WsEvent>) {
    std::thread::spawn(move || {
        let url = ws_url(&server_url, &client_id);
        loop {
            let _connected = run_connection(&url, &tx);
            // Signal disconnected after the connection ends (failed connect or
            // dropped socket). Stop reconnecting if the receiver is gone.
            if tx.send(WsEvent::Disconnected).is_err() {
                return;
            }
            std::thread::sleep(Duration::from_secs(2));
        }
    });
}

/// Returns true if the connection was established at least once before failing.
fn run_connection(url: &str, tx: &Sender<WsEvent>) -> bool {
    let (mut socket, _resp) = match tungstenite::connect(url) {
        Ok(pair) => pair,
        Err(_) => return false,
    };
    // Give the read a timeout so a silent half-open connection is detected and
    // the loop can reconnect (read() would otherwise block forever).
    if let tungstenite::stream::MaybeTlsStream::Plain(s) = socket.get_mut() {
        let _ = s.set_read_timeout(Some(Duration::from_secs(5)));
        let _ = s.set_nodelay(true);
    }
    let _ = tx.send(WsEvent::Connected);

    loop {
        match socket.read() {
            Ok(tungstenite::Message::Text(text)) => {
                if let Some(ev) = parse_message(&text) {
                    if tx.send(ev).is_err() {
                        return true;
                    }
                }
            }
            Ok(tungstenite::Message::Binary(_)) => {}
            Ok(tungstenite::Message::Ping(_)) | Ok(tungstenite::Message::Pong(_)) => {}
            Ok(tungstenite::Message::Close(_)) | Ok(tungstenite::Message::Frame(_)) => return true,
            Err(tungstenite::Error::Io(e))
                if e.kind() == std::io::ErrorKind::TimedOut
                    || e.kind() == std::io::ErrorKind::WouldBlock =>
            {
                continue;
            }
            Err(_) => return true,
        }
    }
}

fn parse_message(text: &str) -> Option<WsEvent> {
    let v: serde_json::Value = serde_json::from_str(text).ok()?;
    let msg_type = v.get("type")?.as_str()?;
    let data = v.get("data")?;
    Some(match msg_type {
        "status" => {
            let q = data.get("status").cloned().unwrap_or(serde_json::Value::Null);
            WsEvent::Status(q)
        }
        "progress" => WsEvent::Progress {
            prompt_id: data.get("prompt_id").and_then(|x| x.as_str()).unwrap_or("").to_string(),
            value: data.get("value").and_then(|x| x.as_f64()).unwrap_or(0.0),
            max: data.get("max").and_then(|x| x.as_f64()).unwrap_or(0.0),
            node: data.get("node").and_then(|x| x.as_str()).unwrap_or("").to_string(),
        },
        "execution_start" => WsEvent::ExecStart {
            prompt_id: data.get("prompt_id").and_then(|x| x.as_str()).unwrap_or("").to_string(),
        },
        "executing" => WsEvent::Executing {
            prompt_id: data.get("prompt_id").and_then(|x| x.as_str()).unwrap_or("").to_string(),
            node: data.get("node").and_then(|x| x.as_str()).map(|s| s.to_string()),
        },
        "execution_success" => WsEvent::ExecSuccess {
            prompt_id: data.get("prompt_id").and_then(|x| x.as_str()).unwrap_or("").to_string(),
            output: data.get("output").cloned().unwrap_or(serde_json::Value::Null),
        },
        "execution_error" => WsEvent::ExecError {
            prompt_id: data.get("prompt_id").and_then(|x| x.as_str()).unwrap_or("").to_string(),
            error: data.get("error").and_then(|x| x.as_str()).unwrap_or("").to_string(),
        },
        _ => return None,
    })
}

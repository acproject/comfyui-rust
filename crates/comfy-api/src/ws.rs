use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{broadcast, Mutex};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct WsMessage {
    #[serde(rename = "type")]
    pub msg_type: String,
    pub data: Value,
}

impl WsMessage {
    pub fn new(msg_type: impl Into<String>, data: Value) -> Self {
        Self {
            msg_type: msg_type.into(),
            data,
        }
    }

    pub fn status(queue_info: Value, sid: &str) -> Self {
        Self::new(
            "status",
            serde_json::json!({
                "status": queue_info,
                "sid": sid,
            }),
        )
    }

    pub fn execution_start(prompt_id: &str) -> Self {
        Self::new(
            "execution_start",
            serde_json::json!({ "prompt_id": prompt_id }),
        )
    }

    pub fn executing(prompt_id: &str, node_id: Option<&str>) -> Self {
        Self::new(
            "executing",
            serde_json::json!({
                "prompt_id": prompt_id,
                "node": node_id,
            }),
        )
    }

    pub fn progress(prompt_id: &str, value: f64, max: f64) -> Self {
        Self::new(
            "progress",
            serde_json::json!({
                "prompt_id": prompt_id,
                "value": value,
                "max": max,
            }),
        )
    }

    pub fn execution_cached(prompt_id: &str, node_ids: &[String]) -> Self {
        Self::new(
            "execution_cached",
            serde_json::json!({
                "prompt_id": prompt_id,
                "nodes": node_ids,
            }),
        )
    }

    pub fn execution_success(prompt_id: &str, output: &Value) -> Self {
        Self::new(
            "execution_success",
            serde_json::json!({
                "prompt_id": prompt_id,
                "output": output,
            }),
        )
    }

    pub fn execution_error(prompt_id: &str, error: &str) -> Self {
        Self::new(
            "execution_error",
            serde_json::json!({
                "prompt_id": prompt_id,
                "error": error,
            }),
        )
    }
}

pub struct WsBroadcaster {
    sender: broadcast::Sender<WsMessage>,
    client_ids: Arc<Mutex<HashMap<String, String>>>,
}

impl WsBroadcaster {
    pub fn new() -> Self {
        let (sender, _) = broadcast::channel(256);
        Self {
            sender,
            client_ids: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn subscribe(&self) -> broadcast::Receiver<WsMessage> {
        self.sender.subscribe()
    }

    pub fn send(&self, msg: WsMessage) {
        let _ = self.sender.send(msg);
    }

    pub async fn register_client(&self, client_id: String, sid: String) {
        self.client_ids.lock().await.insert(client_id, sid);
    }

    pub async fn unregister_client(&self, client_id: &str) {
        self.client_ids.lock().await.remove(client_id);
    }

    pub async fn client_count(&self) -> usize {
        self.client_ids.lock().await.len()
    }
}

impl Clone for WsBroadcaster {
    fn clone(&self) -> Self {
        Self {
            sender: self.sender.clone(),
            client_ids: self.client_ids.clone(),
        }
    }
}

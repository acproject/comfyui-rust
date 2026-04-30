use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{Mutex, Notify};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum JobStatus {
    Pending,
    Running,
    Completed,
    Failed(String),
    Interrupted,
}

#[derive(Debug, Clone)]
pub struct QueueItem {
    pub number: i64,
    pub prompt_id: String,
    pub prompt: HashMap<String, serde_json::Value>,
    pub extra_data: HashMap<String, serde_json::Value>,
    pub client_id: Option<String>,
    pub status: JobStatus,
    pub outputs: HashMap<String, comfy_executor::NodeOutput>,
    pub created_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryEntry {
    pub prompt_id: String,
    pub prompt: HashMap<String, serde_json::Value>,
    pub outputs: HashMap<String, serde_json::Value>,
    pub status: JobStatus,
    pub created_at: u64,
    pub completed_at: Option<u64>,
}

pub struct PromptQueue {
    queue: Arc<Mutex<Vec<QueueItem>>>,
    current: Arc<Mutex<Option<QueueItem>>>,
    history: Arc<Mutex<HashMap<String, HistoryEntry>>>,
    counter: Arc<Mutex<i64>>,
    notify: Arc<Notify>,
    shutdown: Arc<Mutex<bool>>,
}

impl PromptQueue {
    pub fn new() -> Self {
        Self {
            queue: Arc::new(Mutex::new(Vec::new())),
            current: Arc::new(Mutex::new(None)),
            history: Arc::new(Mutex::new(HashMap::new())),
            counter: Arc::new(Mutex::new(0)),
            notify: Arc::new(Notify::new()),
            shutdown: Arc::new(Mutex::new(false)),
        }
    }

    pub async fn put(
        &self,
        prompt: HashMap<String, serde_json::Value>,
        extra_data: HashMap<String, serde_json::Value>,
        client_id: Option<String>,
        front: bool,
    ) -> (String, i64) {
        let mut counter = self.counter.lock().await;
        *counter += 1;
        let number = if front { -*counter } else { *counter };

        let prompt_id = uuid::Uuid::new_v4().to_string();
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        let item = QueueItem {
            number,
            prompt_id: prompt_id.clone(),
            prompt,
            extra_data,
            client_id,
            status: JobStatus::Pending,
            outputs: HashMap::new(),
            created_at: now,
        };

        let mut queue = self.queue.lock().await;
        if front {
            queue.insert(0, item);
        } else {
            queue.push(item);
        }

        self.notify.notify_one();
        (prompt_id, number)
    }

    pub async fn get_next(&self) -> Option<QueueItem> {
        let mut queue = self.queue.lock().await;
        if queue.is_empty() {
            return None;
        }

        let mut item = queue.remove(0);
        item.status = JobStatus::Running;

        let mut current = self.current.lock().await;
        *current = Some(item.clone());

        Some(item)
    }

    pub async fn complete_current(&self, prompt_id: &str, outputs: HashMap<String, comfy_executor::NodeOutput>, status: JobStatus) {
        let mut current = self.current.lock().await;
        if let Some(item) = current.take() {
            if item.prompt_id == prompt_id {
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_millis() as u64;

                let serialized_outputs: HashMap<String, serde_json::Value> = outputs
                    .iter()
                    .map(|(k, v)| (k.clone(), serde_json::to_value(v).unwrap_or(serde_json::Value::Null)))
                    .collect();

                let entry = HistoryEntry {
                    prompt_id: item.prompt_id.clone(),
                    prompt: item.prompt.clone(),
                    outputs: serialized_outputs,
                    status,
                    created_at: item.created_at,
                    completed_at: Some(now),
                };

                let mut history = self.history.lock().await;
                history.insert(item.prompt_id.clone(), entry);
            }
        }
    }

    pub async fn get_queue_info(&self) -> serde_json::Value {
        let queue = self.queue.lock().await;
        let current = self.current.lock().await;

        let running: Vec<serde_json::Value> = if let Some(ref item) = *current {
            vec![serde_json::json!({
                "number": item.number,
                "prompt_id": item.prompt_id,
            })]
        } else {
            vec![]
        };

        let pending: Vec<serde_json::Value> = queue
            .iter()
            .map(|item| {
                serde_json::json!({
                    "number": item.number,
                    "prompt_id": item.prompt_id,
                })
            })
            .collect();

        serde_json::json!({
            "queue_running": running,
            "queue_pending": pending,
        })
    }

    pub async fn get_history(
        &self,
        prompt_id: Option<&str>,
        max_items: Option<usize>,
        offset: Option<usize>,
    ) -> HashMap<String, HistoryEntry> {
        let history = self.history.lock().await;

        if let Some(id) = prompt_id {
            let mut result = HashMap::new();
            if let Some(entry) = history.get(id) {
                result.insert(id.to_string(), entry.clone());
            }
            return result;
        }

        let mut entries: Vec<&HistoryEntry> = history.values().collect();
        entries.sort_by(|a, b| b.completed_at.cmp(&a.completed_at));

        let offset = offset.unwrap_or(0);
        let entries: Vec<&HistoryEntry> = entries.into_iter().skip(offset).collect();

        let entries: Vec<&HistoryEntry> = if let Some(max) = max_items {
            entries.into_iter().take(max).collect()
        } else {
            entries
        };

        entries
            .into_iter()
            .map(|e| (e.prompt_id.clone(), e.clone()))
            .collect()
    }

    pub async fn clear_history(&self) {
        let mut history = self.history.lock().await;
        history.clear();
    }

    pub async fn delete_history_item(&self, prompt_id: &str) {
        let mut history = self.history.lock().await;
        history.remove(prompt_id);
    }

    pub async fn clear_queue(&self) {
        let mut queue = self.queue.lock().await;
        queue.clear();
    }

    pub async fn delete_queue_item(&self, prompt_id: &str) {
        let mut queue = self.queue.lock().await;
        queue.retain(|item| item.prompt_id != prompt_id);
    }

    pub async fn interrupt(&self) -> bool {
        let mut current = self.current.lock().await;
        if let Some(ref mut item) = *current {
            item.status = JobStatus::Interrupted;
            true
        } else {
            false
        }
    }

    pub fn notify(&self) -> Arc<Notify> {
        self.notify.clone()
    }

    pub async fn is_shutdown(&self) -> bool {
        *self.shutdown.lock().await
    }

    pub async fn shutdown(&self) {
        *self.shutdown.lock().await = true;
        self.notify.notify_one();
    }
}

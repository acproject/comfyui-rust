use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadProgress {
    pub id: String,
    pub url: String,
    pub filename: String,
    pub model_type: String,
    pub status: DownloadStatus,
    pub downloaded_bytes: u64,
    pub total_bytes: u64,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum DownloadStatus {
    Pending,
    Downloading,
    Completed,
    Failed,
}

impl DownloadProgress {
    pub fn progress_percent(&self) -> f64 {
        if self.total_bytes == 0 {
            return 0.0;
        }
        (self.downloaded_bytes as f64 / self.total_bytes as f64) * 100.0
    }
}

#[derive(Debug, Clone, Default)]
pub struct DownloadTracker {
    downloads: HashMap<String, DownloadProgress>,
}

impl DownloadTracker {
    pub fn new() -> Self {
        Self {
            downloads: HashMap::new(),
        }
    }

    pub fn add(&mut self, progress: DownloadProgress) {
        self.downloads.insert(progress.id.clone(), progress);
    }

    pub fn update(&mut self, id: &str, status: DownloadStatus, downloaded: u64, total: u64, error: Option<String>) {
        if let Some(p) = self.downloads.get_mut(id) {
            p.status = status;
            p.downloaded_bytes = downloaded;
            p.total_bytes = total;
            p.error = error;
        }
    }

    pub fn get(&self, id: &str) -> Option<&DownloadProgress> {
        self.downloads.get(id)
    }

    pub fn list(&self) -> Vec<DownloadProgress> {
        self.downloads.values().cloned().collect()
    }

    pub fn remove_completed(&mut self) {
        self.downloads.retain(|_, p| p.status != DownloadStatus::Completed && p.status != DownloadStatus::Failed);
    }
}

pub type SharedDownloadTracker = Arc<Mutex<DownloadTracker>>;

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Debug, Clone)]
pub struct ImageEntry {
    pub filename: String,
    pub subfolder: String,
    pub prompt_id: Option<String>,
    pub content_type: String,
}

pub struct ImageStore {
    output_dir: PathBuf,
    images: Arc<Mutex<HashMap<String, ImageEntry>>>,
    counter: Arc<Mutex<u64>>,
}

impl ImageStore {
    pub fn new(output_dir: impl Into<PathBuf>) -> Self {
        let output_dir = output_dir.into();
        std::fs::create_dir_all(&output_dir).ok();
        Self {
            output_dir,
            images: Arc::new(Mutex::new(HashMap::new())),
            counter: Arc::new(Mutex::new(0)),
        }
    }

    pub async fn save_image(
        &self,
        data: &[u8],
        filename: &str,
        subfolder: &str,
        prompt_id: Option<&str>,
    ) -> Result<String, String> {
        let dir = if subfolder.is_empty() {
            self.output_dir.clone()
        } else {
            let dir = self.output_dir.join(subfolder);
            std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
            dir
        };

        let mut counter = self.counter.lock().await;
        *counter += 1;
        let unique_name = if Path::new(filename).exists() {
            format!("{}_{}", counter, filename)
        } else {
            filename.to_string()
        };

        let file_path = dir.join(&unique_name);
        std::fs::write(&file_path, data).map_err(|e| e.to_string())?;

        let image_id = if subfolder.is_empty() {
            unique_name.clone()
        } else {
            format!("{}/{}", subfolder, unique_name)
        };
        let entry = ImageEntry {
            filename: unique_name.clone(),
            subfolder: subfolder.to_string(),
            prompt_id: prompt_id.map(|s| s.to_string()),
            content_type: guess_content_type(&unique_name),
        };

        self.images.lock().await.insert(image_id.clone(), entry);

        Ok(image_id)
    }

    pub async fn get_image(&self, image_id: &str) -> Option<(Vec<u8>, String)> {
        let images = self.images.lock().await;
        let entry = images.get(image_id)?;
        let content_type = entry.content_type.clone();
        let path = if entry.subfolder.is_empty() {
            self.output_dir.join(&entry.filename)
        } else {
            self.output_dir.join(&entry.subfolder).join(&entry.filename)
        };
        drop(images);

        let data = std::fs::read(path).ok()?;
        Some((data, content_type))
    }

    pub async fn list_images(&self, subfolder: Option<&str>) -> Vec<ImageEntry> {
        let images = self.images.lock().await;
        images
            .values()
            .filter(|entry| {
                subfolder
                    .map(|s| entry.subfolder == s)
                    .unwrap_or(true)
            })
            .cloned()
            .collect()
    }

    pub async fn delete_image(&self, image_id: &str) -> bool {
        let mut images = self.images.lock().await;
        if let Some(entry) = images.remove(image_id) {
            let path = if entry.subfolder.is_empty() {
                self.output_dir.join(&entry.filename)
            } else {
                self.output_dir.join(&entry.subfolder).join(&entry.filename)
            };
            std::fs::remove_file(path).is_ok()
        } else {
            false
        }
    }

    pub fn output_dir(&self) -> &Path {
        &self.output_dir
    }
}

fn guess_content_type(filename: &str) -> String {
    let lower = filename.to_lowercase();
    if lower.ends_with(".png") {
        "image/png".to_string()
    } else if lower.ends_with(".jpg") || lower.ends_with(".jpeg") {
        "image/jpeg".to_string()
    } else if lower.ends_with(".webp") {
        "image/webp".to_string()
    } else if lower.ends_with(".gif") {
        "image/gif".to_string()
    } else {
        "application/octet-stream".to_string()
    }
}

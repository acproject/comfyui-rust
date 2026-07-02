use crate::database::{now_iso, AssetRecord, CustomFolder, NewAssetRecord, SharedDatabase};
use std::path::{Path, PathBuf};

/// Manages file-based assets (images, videos, 3D, audio) with SQLite tracking.
///
/// Assets are stored under two root directories:
/// - `input_dir`  – user-uploaded materials
/// - `output_dir` – AI-generated outputs
///
/// Each asset is recorded in the `assets` table with a 1:1 mapping to the file on disk.
pub struct AssetManager {
    db: SharedDatabase,
    input_dir: PathBuf,
    output_dir: PathBuf,
}

/// The source category of an asset.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssetSource {
    Uploaded,
    Generated,
}

impl AssetSource {
    pub fn as_str(&self) -> &'static str {
        match self {
            AssetSource::Uploaded => "uploaded",
            AssetSource::Generated => "generated",
        }
    }
}

/// The type category of an asset.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssetType {
    Image,
    Video,
    Audio,
    Gaussian3D,
}

impl AssetType {
    pub fn as_str(&self) -> &'static str {
        match self {
            AssetType::Image => "image",
            AssetType::Video => "video",
            AssetType::Audio => "audio",
            AssetType::Gaussian3D => "3d",
        }
    }
}

/// Filters for querying assets.
pub struct AssetFilters {
    pub source: Option<String>,
    pub asset_type: Option<String>,
    pub folder_id: Option<i64>,
    pub search: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

impl Default for AssetFilters {
    fn default() -> Self {
        Self {
            source: None,
            asset_type: None,
            folder_id: None,
            search: None,
            limit: Some(500),
            offset: None,
        }
    }
}

impl AssetManager {
    pub fn new(db: SharedDatabase, input_dir: PathBuf, output_dir: PathBuf) -> Self {
        // Ensure sub-directories exist
        Self::ensure_dirs(&input_dir, &[("images", AssetType::Image), ("videos", AssetType::Video), ("audios", AssetType::Audio)]);
        Self::ensure_dirs(&output_dir, &[
            ("images", AssetType::Image),
            ("videos", AssetType::Video),
            ("audios", AssetType::Audio),
            ("3d_gaussians", AssetType::Gaussian3D),
        ]);

        Self { db, input_dir, output_dir }
    }

    fn ensure_dirs(root: &Path, dirs: &[(&str, AssetType)]) {
        for (sub, _) in dirs {
            let _ = std::fs::create_dir_all(root.join(sub));
        }
    }

    pub fn input_dir(&self) -> &Path {
        &self.input_dir
    }

    pub fn output_dir(&self) -> &Path {
        &self.output_dir
    }

    // ---- Type guessing ----

    /// Guess the asset type from a filename's extension.
    pub fn guess_asset_type(filename: &str) -> AssetType {
        let lower = filename.to_lowercase();
        let ext = lower.rsplit('.').next().unwrap_or("");
        match ext {
            "png" | "jpg" | "jpeg" | "webp" | "gif" | "bmp" | "tiff" | "tif" => AssetType::Image,
            "mp4" | "webm" | "avi" | "mov" | "mkv" | "gifv" => AssetType::Video,
            "wav" | "mp3" | "flac" | "ogg" | "aac" | "m4a" => AssetType::Audio,
            "ply" | "splat" => AssetType::Gaussian3D,
            _ => AssetType::Image, // default to image
        }
    }

    /// Guess MIME content type from filename.
    pub fn guess_content_type(filename: &str) -> String {
        let lower = filename.to_lowercase();
        let ext = lower.rsplit('.').next().unwrap_or("");
        match ext {
            "png" => "image/png",
            "jpg" | "jpeg" => "image/jpeg",
            "webp" => "image/webp",
            "gif" => "image/gif",
            "bmp" => "image/bmp",
            "tiff" | "tif" => "image/tiff",
            "mp4" => "video/mp4",
            "webm" => "video/webm",
            "avi" => "video/x-msvideo",
            "mov" => "video/quicktime",
            "mkv" => "video/x-matroska",
            "wav" => "audio/wav",
            "mp3" => "audio/mpeg",
            "flac" => "audio/flac",
            "ogg" => "audio/ogg",
            "aac" => "audio/aac",
            "m4a" => "audio/mp4",
            "ply" => "application/ply",
            "splat" => "application/octet-stream",
            _ => "application/octet-stream",
        }
        .to_string()
    }

    /// Get the sub-directory name for an asset type.
    fn type_subdir(t: AssetType) -> &'static str {
        match t {
            AssetType::Image => "images",
            AssetType::Video => "videos",
            AssetType::Audio => "audios",
            AssetType::Gaussian3D => "3d_gaussians",
        }
    }

    // ---- Upload ----

    /// Save an uploaded file to `input/{type_subdir}/{subfolder}/` and record in DB.
    pub fn save_uploaded_asset(
        &self,
        data: &[u8],
        filename: &str,
        subfolder: &str,
    ) -> Result<i64, String> {
        let asset_type = Self::guess_asset_type(filename);
        let type_dir = Self::type_subdir(asset_type);

        // Build destination directory
        let dir = if subfolder.is_empty() {
            self.input_dir.join(type_dir)
        } else {
            // Sanitize subfolder: no path traversal
            let safe = subfolder
                .split('/')
                .filter(|s| !s.is_empty() && *s != ".." && *s != ".")
                .collect::<Vec<_>>()
                .join("/");
            self.input_dir.join(type_dir).join(&safe)
        };
        std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;

        // Generate unique filename if needed
        let dest = dir.join(filename);
        let final_name = if dest.exists() {
            let stem = Path::new(filename)
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("file");
            let ext = Path::new(filename)
                .extension()
                .and_then(|s| s.to_str())
                .unwrap_or("bin");
            let ts = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis();
            format!("{}_{}.{}", stem, ts, ext)
        } else {
            filename.to_string()
        };
        let dest = dir.join(&final_name);

        std::fs::write(&dest, data).map_err(|e| e.to_string())?;

        let file_size = data.len() as i64;
        let content_type = Self::guess_content_type(&final_name);

        // Build relative path: input/{type_subdir}/{subfolder}/{filename}
        let relative_path = if subfolder.is_empty() {
            format!("input/{}/{final_name}", type_dir)
        } else {
            let safe = subfolder
                .split('/')
                .filter(|s| !s.is_empty() && *s != ".." && *s != ".")
                .collect::<Vec<_>>()
                .join("/");
            format!("input/{}/{safe}/{final_name}", type_dir)
        };

        let now = now_iso();
        let record = NewAssetRecord {
            name: final_name,
            relative_path: relative_path.clone(),
            source: AssetSource::Uploaded.as_str().to_string(),
            asset_type: asset_type.as_str().to_string(),
            subfolder: subfolder.to_string(),
            file_size,
            content_type,
            prompt_id: None,
            workflow_id: None,
            tags: "[]".to_string(),
            custom_folder_id: None,
            meta: "{}".to_string(),
            created_at: now.clone(),
            updated_at: now,
        };

        self.db
            .insert_asset(&record)
            .map_err(|e| format!("DB error: {}", e))
    }

    // ---- Record generated asset ----

    /// Record an AI-generated file that already exists on disk (e.g. saved by the worker).
    ///
    /// `filename` and `subfolder` follow the existing ComfyUI convention where
    /// the file lives under `output_dir/{subfolder}/{filename}`.
    pub fn record_generated_asset(
        &self,
        filename: &str,
        subfolder: &str,
        prompt_id: Option<&str>,
    ) -> Result<i64, String> {
        let asset_type = Self::guess_asset_type(filename);

        // Build the absolute path to check file existence and get size
        let abs_path = if subfolder.is_empty() {
            self.output_dir.join(filename)
        } else {
            self.output_dir.join(subfolder).join(filename)
        };

        if !abs_path.exists() {
            return Err(format!("File not found: {}", abs_path.display()));
        }

        let file_size = std::fs::metadata(&abs_path)
            .map(|m| m.len() as i64)
            .unwrap_or(0);
        let content_type = Self::guess_content_type(filename);

        // Build relative path
        let relative_path = if subfolder.is_empty() {
            format!("output/{}", filename)
        } else {
            format!("output/{}/{}", subfolder, filename)
        };

        let now = now_iso();
        let record = NewAssetRecord {
            name: filename.to_string(),
            relative_path: relative_path.clone(),
            source: AssetSource::Generated.as_str().to_string(),
            asset_type: asset_type.as_str().to_string(),
            subfolder: subfolder.to_string(),
            file_size,
            content_type,
            prompt_id: prompt_id.map(|s| s.to_string()),
            workflow_id: None,
            tags: "[]".to_string(),
            custom_folder_id: None,
            meta: "{}".to_string(),
            created_at: now.clone(),
            updated_at: now,
        };

        self.db
            .insert_asset(&record)
            .map_err(|e| format!("DB error: {}", e))
    }

    // ---- Query ----

    pub fn list_assets(&self, filters: &AssetFilters) -> Result<Vec<AssetRecord>, String> {
        self.db
            .list_assets(
                filters.source.as_deref(),
                filters.asset_type.as_deref(),
                filters.folder_id,
                filters.search.as_deref(),
                filters.limit,
                filters.offset,
            )
            .map_err(|e| format!("DB error: {}", e))
    }

    pub fn get_asset(&self, id: i64) -> Result<Option<AssetRecord>, String> {
        self.db
            .get_asset(id)
            .map_err(|e| format!("DB error: {}", e))
    }

    // ---- Delete ----

    pub fn delete_asset(&self, id: i64) -> Result<(), String> {
        let record = self
            .db
            .get_asset(id)
            .map_err(|e| format!("DB error: {}", e))?
            .ok_or_else(|| format!("Asset {} not found", id))?;

        // Delete the file from disk
        let abs_path = self.resolve_abs_path(&record.relative_path);
        if let Some(path) = abs_path {
            if path.exists() {
                std::fs::remove_file(&path).map_err(|e| e.to_string())?;
            }
        }

        // Delete from DB
        self.db
            .delete_asset(id)
            .map_err(|e| format!("DB error: {}", e))
    }

    // ---- Update ----

    pub fn rename_asset(&self, id: i64, new_name: &str) -> Result<(), String> {
        let record = self
            .db
            .get_asset(id)
            .map_err(|e| format!("DB error: {}", e))?
            .ok_or_else(|| format!("Asset {} not found", id))?;

        // Rename file on disk
        let old_path = self
            .resolve_abs_path(&record.relative_path)
            .ok_or_else(|| "Cannot resolve path".to_string())?;
        let parent = old_path.parent().ok_or_else(|| "No parent dir".to_string())?;
        let new_path = parent.join(new_name);

        if old_path != new_path {
            if new_path.exists() {
                return Err(format!("File already exists: {}", new_name));
            }
            std::fs::rename(&old_path, &new_path).map_err(|e| e.to_string())?;
        }

        // Update relative path in DB
        let new_relative = build_relative_path(&record.relative_path, new_name);
        // We update name and relative_path via a raw SQL since update_asset only does name/tags/folder
        self.update_relative_path(id, new_name, &new_relative)?;

        Ok(())
    }

    pub fn update_tags(&self, id: i64, tags: &str) -> Result<(), String> {
        self.db
            .update_asset(id, None, Some(tags), None)
            .map_err(|e| format!("DB error: {}", e))
    }

    pub fn move_to_folder(&self, id: i64, folder_id: Option<i64>) -> Result<(), String> {
        self.db
            .update_asset(id, None, None, Some(folder_id))
            .map_err(|e| format!("DB error: {}", e))
    }

    // ---- Custom folders ----

    pub fn create_folder(&self, name: &str, parent_id: Option<i64>, color: &str) -> Result<i64, String> {
        self.db
            .create_folder(name, parent_id, color)
            .map_err(|e| format!("DB error: {}", e))
    }

    pub fn list_folders(&self) -> Result<Vec<CustomFolder>, String> {
        self.db
            .list_folders()
            .map_err(|e| format!("DB error: {}", e))
    }

    pub fn delete_folder(&self, id: i64) -> Result<(), String> {
        self.db
            .delete_folder(id)
            .map_err(|e| format!("DB error: {}", e))
    }

    // ---- Scan & sync ----

    /// Scan the input and output directories for files not yet in the DB,
    /// and insert them. Returns the number of newly registered assets.
    pub fn scan_and_sync(&self) -> Result<usize, String> {
        let mut count = 0;
        count += self.scan_dir(&self.input_dir, AssetSource::Uploaded, "input")?;
        count += self.scan_dir(&self.output_dir, AssetSource::Generated, "output")?;
        tracing::info!("Asset scan complete: {} new assets registered", count);
        Ok(count)
    }

    fn scan_dir(
        &self,
        root: &Path,
        source: AssetSource,
        prefix: &str,
    ) -> Result<usize, String> {
        let mut count = 0;
        if !root.exists() {
            return Ok(0);
        }

        let mut stack = vec![root.to_path_buf()];
        while let Some(dir) = stack.pop() {
            let entries = match std::fs::read_dir(&dir) {
                Ok(e) => e,
                Err(_) => continue,
            };
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    stack.push(path);
                } else if path.is_file() {
                    if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                        // Skip hidden files
                        if name.starts_with('.') {
                            continue;
                        }
                        let asset_type = Self::guess_asset_type(name);
                        // Build relative path
                        if let Ok(rel) = path.strip_prefix(root) {
                            let relative_path = format!("{}/{}", prefix, rel.to_string_lossy());
                            // Check if already in DB
                            let existing = self
                                .db
                                .get_asset_by_path(&relative_path)
                                .map_err(|e| format!("DB error: {}", e))?;
                            if existing.is_none() {
                                let file_size = std::fs::metadata(&path)
                                    .map(|m| m.len() as i64)
                                    .unwrap_or(0);
                                let content_type = Self::guess_content_type(name);
                                let subfolder = rel
                                    .parent()
                                    .map(|p| p.to_string_lossy().to_string())
                                    .unwrap_or_default();

                                let now = now_iso();
                                let record = NewAssetRecord {
                                    name: name.to_string(),
                                    relative_path,
                                    source: source.as_str().to_string(),
                                    asset_type: asset_type.as_str().to_string(),
                                    subfolder,
                                    file_size,
                                    content_type,
                                    prompt_id: None,
                                    workflow_id: None,
                                    tags: "[]".to_string(),
                                    custom_folder_id: None,
                                    meta: "{}".to_string(),
                                    created_at: now.clone(),
                                    updated_at: now,
                                };
                                if let Ok(_) = self.db.insert_asset(&record) {
                                    count += 1;
                                }
                            }
                        }
                    }
                }
            }
        }
        Ok(count)
    }

    // ---- Helpers ----

    /// Resolve a relative path (e.g. "input/images/foo.png") to an absolute path.
    pub fn resolve_abs_path(&self, relative_path: &str) -> Option<PathBuf> {
        let p = Path::new(relative_path);
        // Check if it starts with "input/" or "output/"
        let first = p.components().next()?;
        match first.as_os_str().to_str()? {
            "input" => Some(self.input_dir.join(p.strip_prefix("input").ok()?)),
            "output" => Some(self.output_dir.join(p.strip_prefix("output").ok()?)),
            _ => None,
        }
    }

    /// Update the relative_path and name of an asset (used internally by rename).
    fn update_relative_path(
        &self,
        id: i64,
        new_name: &str,
        new_relative_path: &str,
    ) -> Result<(), String> {
        let now = now_iso();
        let params: &[&dyn rusqlite::ToSql] = &[
            &new_name,
            &new_relative_path,
            &now,
            &id,
        ];
        self.db
            .execute_raw(
                "UPDATE assets SET name = ?1, relative_path = ?2, updated_at = ?3 WHERE id = ?4",
                params,
            )
            .map_err(|e| format!("DB error: {}", e))?;
        Ok(())
    }
}

/// Build a new relative path by replacing the filename in an existing relative path.
fn build_relative_path(old_relative: &str, new_name: &str) -> String {
    if let Some(idx) = old_relative.rfind('/') {
        format!("{}{}", &old_relative[..=idx], new_name)
    } else {
        new_name.to_string()
    }
}

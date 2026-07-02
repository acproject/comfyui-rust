use rusqlite::{params, Connection};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::sync::Mutex;
use tracing;

pub struct Database {
    conn: Mutex<Connection>,
}

impl Database {
    pub fn open(path: &Path) -> Result<Self, DatabaseError> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(DatabaseError::Io)?;
        }

        let conn = Connection::open(path).map_err(DatabaseError::Sqlite)?;

        Self::init_tables(&conn)?;

        tracing::info!("Database opened at {}", path.display());

        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    pub fn open_in_memory() -> Result<Self, DatabaseError> {
        let conn = Connection::open_in_memory().map_err(DatabaseError::Sqlite)?;

        Self::init_tables(&conn)?;

        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    fn init_tables(conn: &Connection) -> Result<(), DatabaseError> {
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS kv_store (
                key   TEXT PRIMARY KEY,
                value TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS custom_folders (
                id         INTEGER PRIMARY KEY AUTOINCREMENT,
                name       TEXT NOT NULL,
                parent_id  INTEGER,
                color      TEXT DEFAULT '',
                created_at TEXT NOT NULL,
                FOREIGN KEY (parent_id) REFERENCES custom_folders(id) ON DELETE CASCADE
            );
            CREATE TABLE IF NOT EXISTS assets (
                id               INTEGER PRIMARY KEY AUTOINCREMENT,
                name             TEXT NOT NULL,
                relative_path    TEXT NOT NULL,
                source           TEXT NOT NULL,
                asset_type       TEXT NOT NULL,
                subfolder        TEXT DEFAULT '',
                file_size        INTEGER DEFAULT 0,
                content_type     TEXT DEFAULT '',
                prompt_id        TEXT,
                workflow_id      TEXT,
                tags             TEXT DEFAULT '[]',
                custom_folder_id INTEGER,
                meta             TEXT DEFAULT '{}',
                created_at       TEXT NOT NULL,
                updated_at       TEXT NOT NULL,
                FOREIGN KEY (custom_folder_id) REFERENCES custom_folders(id) ON DELETE SET NULL
            );
            CREATE INDEX IF NOT EXISTS idx_assets_source ON assets(source);
            CREATE INDEX IF NOT EXISTS idx_assets_type   ON assets(asset_type);
            CREATE INDEX IF NOT EXISTS idx_assets_folder ON assets(custom_folder_id);
            CREATE UNIQUE INDEX IF NOT EXISTS idx_assets_rel_path ON assets(relative_path);
            "
        ).map_err(DatabaseError::Sqlite)?;
        Ok(())
    }

    pub fn get<T: DeserializeOwned>(&self, key: &str) -> Result<Option<T>, DatabaseError> {
        let conn = self.conn.lock().map_err(|_| DatabaseError::LockError)?;
        let mut stmt = conn.prepare("SELECT value FROM kv_store WHERE key = ?1")
            .map_err(DatabaseError::Sqlite)?;

        let result = stmt.query_row(params![key], |row| {
            let value: String = row.get(0)?;
            Ok(value)
        });

        match result {
            Ok(json_str) => {
                let value: T = serde_json::from_str(&json_str)
                    .map_err(DatabaseError::Json)?;
                Ok(Some(value))
            }
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(DatabaseError::Sqlite(e)),
        }
    }

    pub fn set<T: Serialize>(&self, key: &str, value: &T) -> Result<(), DatabaseError> {
        let conn = self.conn.lock().map_err(|_| DatabaseError::LockError)?;
        let json = serde_json::to_string(value).map_err(DatabaseError::Json)?;

        conn.execute(
            "INSERT OR REPLACE INTO kv_store (key, value) VALUES (?1, ?2)",
            params![key, json],
        ).map_err(DatabaseError::Sqlite)?;

        Ok(())
    }

    pub fn delete(&self, key: &str) -> Result<(), DatabaseError> {
        let conn = self.conn.lock().map_err(|_| DatabaseError::LockError)?;
        conn.execute("DELETE FROM kv_store WHERE key = ?1", params![key])
            .map_err(DatabaseError::Sqlite)?;
        Ok(())
    }

    pub fn get_raw(&self, key: &str) -> Result<Option<String>, DatabaseError> {
        let conn = self.conn.lock().map_err(|_| DatabaseError::LockError)?;
        let mut stmt = conn.prepare("SELECT value FROM kv_store WHERE key = ?1")
            .map_err(DatabaseError::Sqlite)?;

        let result = stmt.query_row(params![key], |row| {
            let value: String = row.get(0)?;
            Ok(value)
        });

        match result {
            Ok(v) => Ok(Some(v)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(DatabaseError::Sqlite(e)),
        }
    }

    // ---- Asset CRUD ----

    pub fn insert_asset(&self, record: &NewAssetRecord) -> Result<i64, DatabaseError> {
        let conn = self.conn.lock().map_err(|_| DatabaseError::LockError)?;
        conn.execute(
            "INSERT OR IGNORE INTO assets
             (name, relative_path, source, asset_type, subfolder, file_size, content_type,
              prompt_id, workflow_id, tags, custom_folder_id, meta, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)",
            params![
                record.name,
                record.relative_path,
                record.source,
                record.asset_type,
                record.subfolder,
                record.file_size,
                record.content_type,
                record.prompt_id,
                record.workflow_id,
                record.tags,
                record.custom_folder_id,
                record.meta,
                record.created_at,
                record.updated_at,
            ],
        ).map_err(DatabaseError::Sqlite)?;
        let id = conn.last_insert_rowid();
        // If INSERT OR IGNORE skipped (row already exists), find existing id
        if id == 0 {
            let existing: Option<i64> = conn.query_row(
                "SELECT id FROM assets WHERE relative_path = ?1",
                params![record.relative_path],
                |row| row.get(0),
            ).ok();
            Ok(existing.unwrap_or(0))
        } else {
            Ok(id)
        }
    }

    pub fn get_asset(&self, id: i64) -> Result<Option<AssetRecord>, DatabaseError> {
        let conn = self.conn.lock().map_err(|_| DatabaseError::LockError)?;
        let result = conn.query_row(
            "SELECT id, name, relative_path, source, asset_type, subfolder,
                    file_size, content_type, prompt_id, workflow_id, tags,
                    custom_folder_id, meta, created_at, updated_at
             FROM assets WHERE id = ?1",
            params![id],
            |row| {
                Ok(AssetRecord {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    relative_path: row.get(2)?,
                    source: row.get(3)?,
                    asset_type: row.get(4)?,
                    subfolder: row.get(5)?,
                    file_size: row.get(6)?,
                    content_type: row.get(7)?,
                    prompt_id: row.get(8)?,
                    workflow_id: row.get(9)?,
                    tags: row.get(10)?,
                    custom_folder_id: row.get(11)?,
                    meta: row.get(12)?,
                    created_at: row.get(13)?,
                    updated_at: row.get(14)?,
                })
            },
        );
        match result {
            Ok(r) => Ok(Some(r)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(DatabaseError::Sqlite(e)),
        }
    }

    pub fn get_asset_by_path(&self, relative_path: &str) -> Result<Option<AssetRecord>, DatabaseError> {
        let conn = self.conn.lock().map_err(|_| DatabaseError::LockError)?;
        let result = conn.query_row(
            "SELECT id, name, relative_path, source, asset_type, subfolder,
                    file_size, content_type, prompt_id, workflow_id, tags,
                    custom_folder_id, meta, created_at, updated_at
             FROM assets WHERE relative_path = ?1",
            params![relative_path],
            |row| {
                Ok(AssetRecord {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    relative_path: row.get(2)?,
                    source: row.get(3)?,
                    asset_type: row.get(4)?,
                    subfolder: row.get(5)?,
                    file_size: row.get(6)?,
                    content_type: row.get(7)?,
                    prompt_id: row.get(8)?,
                    workflow_id: row.get(9)?,
                    tags: row.get(10)?,
                    custom_folder_id: row.get(11)?,
                    meta: row.get(12)?,
                    created_at: row.get(13)?,
                    updated_at: row.get(14)?,
                })
            },
        );
        match result {
            Ok(r) => Ok(Some(r)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(DatabaseError::Sqlite(e)),
        }
    }

    pub fn list_assets(
        &self,
        source: Option<&str>,
        asset_type: Option<&str>,
        folder_id: Option<i64>,
        search: Option<&str>,
        limit: Option<i64>,
        offset: Option<i64>,
    ) -> Result<Vec<AssetRecord>, DatabaseError> {
        let conn = self.conn.lock().map_err(|_| DatabaseError::LockError)?;
        let mut sql = String::from(
            "SELECT id, name, relative_path, source, asset_type, subfolder,
                    file_size, content_type, prompt_id, workflow_id, tags,
                    custom_folder_id, meta, created_at, updated_at
             FROM assets WHERE 1=1",
        );
        let mut param_values: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();
        let mut param_idx = 1;

        if let Some(s) = source {
            sql.push_str(&format!(" AND source = ?{}", param_idx));
            param_values.push(Box::new(s.to_string()));
            param_idx += 1;
        }
        if let Some(t) = asset_type {
            sql.push_str(&format!(" AND asset_type = ?{}", param_idx));
            param_values.push(Box::new(t.to_string()));
            param_idx += 1;
        }
        if let Some(fid) = folder_id {
            sql.push_str(&format!(" AND custom_folder_id = ?{}", param_idx));
            param_values.push(Box::new(fid));
            param_idx += 1;
        }
        if let Some(q) = search {
            sql.push_str(&format!(" AND (name LIKE ?{} OR tags LIKE ?{})", param_idx, param_idx));
            param_values.push(Box::new(format!("%{}%", q)));
            param_idx += 1;
        }
        sql.push_str(" ORDER BY created_at DESC");
        if let Some(l) = limit {
            sql.push_str(&format!(" LIMIT ?{}", param_idx));
            param_values.push(Box::new(l));
            param_idx += 1;
            if let Some(o) = offset {
                sql.push_str(&format!(" OFFSET ?{}", param_idx));
                param_values.push(Box::new(o));
            }
        }

        let params_ref: Vec<&dyn rusqlite::ToSql> = param_values.iter().map(|p| p.as_ref()).collect();
        let mut stmt = conn.prepare(&sql).map_err(DatabaseError::Sqlite)?;
        let rows = stmt.query_map(params_ref.as_slice(), |row| {
            Ok(AssetRecord {
                id: row.get(0)?,
                name: row.get(1)?,
                relative_path: row.get(2)?,
                source: row.get(3)?,
                asset_type: row.get(4)?,
                subfolder: row.get(5)?,
                file_size: row.get(6)?,
                content_type: row.get(7)?,
                prompt_id: row.get(8)?,
                workflow_id: row.get(9)?,
                tags: row.get(10)?,
                custom_folder_id: row.get(11)?,
                meta: row.get(12)?,
                created_at: row.get(13)?,
                updated_at: row.get(14)?,
            })
        }).map_err(DatabaseError::Sqlite)?;

        let mut results = Vec::new();
        for row in rows {
            results.push(row.map_err(DatabaseError::Sqlite)?);
        }
        Ok(results)
    }

    pub fn count_assets(&self) -> Result<i64, DatabaseError> {
        let conn = self.conn.lock().map_err(|_| DatabaseError::LockError)?;
        let count: i64 = conn.query_row("SELECT COUNT(*) FROM assets", [], |row| row.get(0))
            .map_err(DatabaseError::Sqlite)?;
        Ok(count)
    }

    pub fn update_asset(
        &self,
        id: i64,
        name: Option<&str>,
        tags: Option<&str>,
        custom_folder_id: Option<Option<i64>>,
    ) -> Result<(), DatabaseError> {
        let conn = self.conn.lock().map_err(|_| DatabaseError::LockError)?;
        let now = now_iso();
        let mut updates: Vec<String> = Vec::new();
        let mut params_vec: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();
        let mut idx = 1;

        if let Some(n) = name {
            updates.push(format!("name = ?{}", idx));
            params_vec.push(Box::new(n.to_string()));
            idx += 1;
        }
        if let Some(t) = tags {
            updates.push(format!("tags = ?{}", idx));
            params_vec.push(Box::new(t.to_string()));
            idx += 1;
        }
        if let Some(fid) = custom_folder_id {
            updates.push(format!("custom_folder_id = ?{}", idx));
            params_vec.push(Box::new(fid));
            idx += 1;
        }
        updates.push(format!("updated_at = ?{}", idx));
        params_vec.push(Box::new(now));
        idx += 1;

        params_vec.push(Box::new(id));
        let sql = format!("UPDATE assets SET {} WHERE id = ?{}", updates.join(", "), idx);
        let params_ref: Vec<&dyn rusqlite::ToSql> = params_vec.iter().map(|p| p.as_ref()).collect();
        conn.execute(&sql, params_ref.as_slice()).map_err(DatabaseError::Sqlite)?;
        Ok(())
    }

    pub fn delete_asset(&self, id: i64) -> Result<(), DatabaseError> {
        let conn = self.conn.lock().map_err(|_| DatabaseError::LockError)?;
        conn.execute("DELETE FROM assets WHERE id = ?1", params![id])
            .map_err(DatabaseError::Sqlite)?;
        Ok(())
    }

    // ---- Custom Folder CRUD ----

    pub fn create_folder(&self, name: &str, parent_id: Option<i64>, color: &str) -> Result<i64, DatabaseError> {
        let conn = self.conn.lock().map_err(|_| DatabaseError::LockError)?;
        let now = now_iso();
        conn.execute(
            "INSERT INTO custom_folders (name, parent_id, color, created_at) VALUES (?1, ?2, ?3, ?4)",
            params![name, parent_id, color, now],
        ).map_err(DatabaseError::Sqlite)?;
        Ok(conn.last_insert_rowid())
    }

    pub fn list_folders(&self) -> Result<Vec<CustomFolder>, DatabaseError> {
        let conn = self.conn.lock().map_err(|_| DatabaseError::LockError)?;
        let mut stmt = conn.prepare(
            "SELECT id, name, parent_id, color, created_at FROM custom_folders ORDER BY name",
        ).map_err(DatabaseError::Sqlite)?;
        let rows = stmt.query_map([], |row| {
            Ok(CustomFolder {
                id: row.get(0)?,
                name: row.get(1)?,
                parent_id: row.get(2)?,
                color: row.get(3)?,
                created_at: row.get(4)?,
            })
        }).map_err(DatabaseError::Sqlite)?;

        let mut results = Vec::new();
        for row in rows {
            results.push(row.map_err(DatabaseError::Sqlite)?);
        }
        Ok(results)
    }

    pub fn delete_folder(&self, id: i64) -> Result<(), DatabaseError> {
        let conn = self.conn.lock().map_err(|_| DatabaseError::LockError)?;
        // First set custom_folder_id to NULL for assets in this folder
        conn.execute(
            "UPDATE assets SET custom_folder_id = NULL WHERE custom_folder_id = ?1",
            params![id],
        ).map_err(DatabaseError::Sqlite)?;
        conn.execute("DELETE FROM custom_folders WHERE id = ?1", params![id])
            .map_err(DatabaseError::Sqlite)?;
        Ok(())
    }

    /// Execute a raw SQL statement with parameters (for internal use by AssetManager).
    pub fn execute_raw(
        &self,
        sql: &str,
        params: &[&dyn rusqlite::ToSql],
    ) -> Result<usize, DatabaseError> {
        let conn = self.conn.lock().map_err(|_| DatabaseError::LockError)?;
        conn.execute(sql, params).map_err(DatabaseError::Sqlite)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum DatabaseError {
    #[error("SQLite error: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Lock error")]
    LockError,
}

pub type SharedDatabase = std::sync::Arc<Database>;

// ---- Data models ----

/// Full asset record as stored in the database.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetRecord {
    pub id: i64,
    pub name: String,
    pub relative_path: String,
    pub source: String,
    pub asset_type: String,
    pub subfolder: String,
    pub file_size: i64,
    pub content_type: String,
    pub prompt_id: Option<String>,
    pub workflow_id: Option<String>,
    pub tags: String,
    pub custom_folder_id: Option<i64>,
    pub meta: String,
    pub created_at: String,
    pub updated_at: String,
}

/// Data needed to insert a new asset (no `id` or auto-managed fields).
#[derive(Debug, Clone)]
pub struct NewAssetRecord {
    pub name: String,
    pub relative_path: String,
    pub source: String,
    pub asset_type: String,
    pub subfolder: String,
    pub file_size: i64,
    pub content_type: String,
    pub prompt_id: Option<String>,
    pub workflow_id: Option<String>,
    pub tags: String,
    pub custom_folder_id: Option<i64>,
    pub meta: String,
    pub created_at: String,
    pub updated_at: String,
}

/// A user-created custom folder for organizing assets.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomFolder {
    pub id: i64,
    pub name: String,
    pub parent_id: Option<i64>,
    pub color: String,
    pub created_at: String,
}

/// Generate an ISO-8601 timestamp in UTC.
pub fn now_iso() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let dur = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default();
    let secs = dur.as_secs();
    let days = secs / 86400;
    let rem = secs % 86400;
    let hour = rem / 3600;
    let min = (rem % 3600) / 60;
    let sec = rem % 60;
    // Days since 1970-01-01 -> convert to Y-M-D (Howard Hinnant algorithm)
    let z = days as i64 + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = (z - era * 146097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u64;
    let year = if m <= 2 { y + 1 } else { y };
    format!("{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z", year, m, d, hour, min, sec)
}

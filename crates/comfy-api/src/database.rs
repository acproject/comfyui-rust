use rusqlite::{params, Connection};
use serde::de::DeserializeOwned;
use serde::Serialize;
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

        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS kv_store (
                key   TEXT PRIMARY KEY,
                value TEXT NOT NULL
            );"
        ).map_err(DatabaseError::Sqlite)?;

        tracing::info!("Database opened at {}", path.display());

        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    pub fn open_in_memory() -> Result<Self, DatabaseError> {
        let conn = Connection::open_in_memory().map_err(DatabaseError::Sqlite)?;

        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS kv_store (
                key   TEXT PRIMARY KEY,
                value TEXT NOT NULL
            );"
        ).map_err(DatabaseError::Sqlite)?;

        Ok(Self {
            conn: Mutex::new(conn),
        })
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

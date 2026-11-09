//! SQLite persistence for AI Agent conversations.
//!
//! The storage model follows the standard session/message split:
//! - [`agent_sessions`](AgentSession) groups one conversation and holds a
//!   user-visible title plus free-form JSON metadata.
//! - [`agent_messages`](AgentMessageRecord) stores every turn in insertion
//!   order; rows are removed automatically when their session is deleted
//!   (`ON DELETE CASCADE`, requires `PRAGMA foreign_keys = ON`, which
//!   [`Database::open`](crate::database::Database::open) enables).
//!
//! These methods extend the shared [`Database`] so callers keep using the
//! single `state.db` handle rather than opening a second connection.

use crate::database::{now_iso, Database, DatabaseError};
use rusqlite::params;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// One persisted AI Agent conversation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentSession {
    pub id: String,
    pub title: String,
    /// Free-form JSON object (defaults to `{}`).
    pub meta: String,
    pub created_at: String,
    pub updated_at: String,
    /// Number of stored messages; populated on list/get, ignored on insert.
    #[serde(default)]
    pub message_count: i64,
}

/// One persisted chat turn. `actions` is a JSON array string of
/// [`crate::agent::AgentAction`] values (empty array `[]` for plain messages).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentMessageRecord {
    pub id: i64,
    pub session_id: String,
    pub role: String,
    pub content: String,
    pub actions: String,
    pub created_at: String,
}

impl Database {
    /// Create a new empty session. When `title` is `None`/empty a placeholder
    /// is stored; callers can rename it once the first user message arrives.
    pub fn create_agent_session(
        &self,
        title: Option<&str>,
        meta: Option<&str>,
    ) -> Result<AgentSession, DatabaseError> {
        let id = Uuid::new_v4().to_string();
        let now = now_iso();
        let title = title.unwrap_or("").trim();
        let meta = meta.unwrap_or("{}");
        let conn = self.conn.lock().map_err(|_| DatabaseError::LockError)?;
        conn.execute(
            "INSERT INTO agent_sessions (id, title, meta, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![id, title, meta, now, now],
        )
        .map_err(DatabaseError::Sqlite)?;
        Ok(AgentSession {
            id,
            title: title.to_string(),
            meta: meta.to_string(),
            created_at: now.clone(),
            updated_at: now,
            message_count: 0,
        })
    }

    /// Return a session by id, including its current message count.
    pub fn get_agent_session(&self, id: &str) -> Result<Option<AgentSession>, DatabaseError> {
        let conn = self.conn.lock().map_err(|_| DatabaseError::LockError)?;
        let result = conn.query_row(
            "SELECT s.id, s.title, s.meta, s.created_at, s.updated_at,
                    (SELECT COUNT(*) FROM agent_messages m WHERE m.session_id = s.id)
             FROM agent_sessions s WHERE s.id = ?1",
            params![id],
            map_session_row,
        );
        match result {
            Ok(s) => Ok(Some(s)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(DatabaseError::Sqlite(e)),
        }
    }

    /// List sessions newest-first (by last activity), each with message count.
    ///
    /// `updated_at` has second-level granularity, so ties are broken by the
    /// latest message id (most recently active conversation wins) and then
    /// session creation order — the result is fully deterministic.
    pub fn list_agent_sessions(&self, limit: Option<i64>) -> Result<Vec<AgentSession>, DatabaseError> {
        let conn = self.conn.lock().map_err(|_| DatabaseError::LockError)?;
        let limit = limit.unwrap_or(100).max(1);
        let mut stmt = conn
            .prepare(
                "SELECT s.id, s.title, s.meta, s.created_at, s.updated_at,
                        (SELECT COUNT(*) FROM agent_messages m WHERE m.session_id = s.id)
                 FROM agent_sessions s
                 ORDER BY s.updated_at DESC,
                          COALESCE((SELECT MAX(id) FROM agent_messages m
                                    WHERE m.session_id = s.id), 0) DESC,
                          s.rowid DESC
                 LIMIT ?1",
            )
            .map_err(DatabaseError::Sqlite)?;
        let rows = stmt
            .query_map(params![limit], map_session_row)
            .map_err(DatabaseError::Sqlite)?;
        let mut out = Vec::new();
        for row in rows {
            out.push(row.map_err(DatabaseError::Sqlite)?);
        }
        Ok(out)
    }

    /// Rename a session. Returns `false` when no row matched the id.
    pub fn rename_agent_session(&self, id: &str, title: &str) -> Result<bool, DatabaseError> {
        let conn = self.conn.lock().map_err(|_| DatabaseError::LockError)?;
        let affected = conn
            .execute(
                "UPDATE agent_sessions SET title = ?2, updated_at = ?3 WHERE id = ?1",
                params![id, title.trim(), now_iso()],
            )
            .map_err(DatabaseError::Sqlite)?;
        Ok(affected > 0)
    }

    /// Bump `updated_at` so active conversations float to the top of the list.
    pub fn touch_agent_session(&self, id: &str) -> Result<(), DatabaseError> {
        let conn = self.conn.lock().map_err(|_| DatabaseError::LockError)?;
        conn.execute(
            "UPDATE agent_sessions SET updated_at = ?2 WHERE id = ?1",
            params![id, now_iso()],
        )
        .map_err(DatabaseError::Sqlite)?;
        Ok(())
    }

    /// Delete a session and — via `ON DELETE CASCADE` — all of its messages.
    pub fn delete_agent_session(&self, id: &str) -> Result<(), DatabaseError> {
        let conn = self.conn.lock().map_err(|_| DatabaseError::LockError)?;
        conn.execute("DELETE FROM agent_sessions WHERE id = ?1", params![id])
            .map_err(DatabaseError::Sqlite)?;
        Ok(())
    }

    /// Append a message and bump the parent session's `updated_at`.
    ///
    /// `actions_json` must be a serialized JSON array; pass `"[]"` for plain
    /// turns. Fails with a foreign-key error when the session does not exist.
    pub fn add_agent_message(
        &self,
        session_id: &str,
        role: &str,
        content: &str,
        actions_json: &str,
    ) -> Result<AgentMessageRecord, DatabaseError> {
        let now = now_iso();
        let conn = self.conn.lock().map_err(|_| DatabaseError::LockError)?;
        conn.execute(
            "INSERT INTO agent_messages (session_id, role, content, actions, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![session_id, role, content, actions_json, now],
        )
        .map_err(DatabaseError::Sqlite)?;
        let id = conn.last_insert_rowid();
        // Keep the session at the top of the recency ordering.
        conn.execute(
            "UPDATE agent_sessions SET updated_at = ?2 WHERE id = ?1",
            params![session_id, now],
        )
        .map_err(DatabaseError::Sqlite)?;
        Ok(AgentMessageRecord {
            id,
            session_id: session_id.to_string(),
            role: role.to_string(),
            content: content.to_string(),
            actions: actions_json.to_string(),
            created_at: now,
        })
    }

    /// Load a session's messages in chronological order. `limit` caps the
    /// most-recent window but the result is always returned oldest-first so
    /// callers can feed it straight into an LLM request.
    pub fn list_agent_messages(
        &self,
        session_id: &str,
        limit: Option<i64>,
    ) -> Result<Vec<AgentMessageRecord>, DatabaseError> {
        let conn = self.conn.lock().map_err(|_| DatabaseError::LockError)?;
        let sql = match limit {
            Some(_) => {
                "SELECT id, session_id, role, content, actions, created_at
                 FROM (
                     SELECT * FROM agent_messages
                     WHERE session_id = ?1
                     ORDER BY id DESC
                     LIMIT ?2
                 ) ORDER BY id ASC"
            }
            None => {
                "SELECT id, session_id, role, content, actions, created_at
                 FROM agent_messages WHERE session_id = ?1 ORDER BY id ASC"
            }
        };
        let mut stmt = conn.prepare(sql).map_err(DatabaseError::Sqlite)?;
        let rows = if let Some(n) = limit {
            stmt.query_map(params![session_id, n], map_message_row)
        } else {
            stmt.query_map(params![session_id], map_message_row)
        }
        .map_err(DatabaseError::Sqlite)?;
        let mut out = Vec::new();
        for row in rows {
            out.push(row.map_err(DatabaseError::Sqlite)?);
        }
        Ok(out)
    }

    /// Count messages belonging to a session.
    pub fn count_agent_messages(&self, session_id: &str) -> Result<i64, DatabaseError> {
        let conn = self.conn.lock().map_err(|_| DatabaseError::LockError)?;
        conn.query_row(
            "SELECT COUNT(*) FROM agent_messages WHERE session_id = ?1",
            params![session_id],
            |row| row.get(0),
        )
        .map_err(DatabaseError::Sqlite)
    }
}

fn map_session_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<AgentSession> {
    Ok(AgentSession {
        id: row.get(0)?,
        title: row.get(1)?,
        meta: row.get(2)?,
        created_at: row.get(3)?,
        updated_at: row.get(4)?,
        message_count: row.get(5)?,
    })
}

fn map_message_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<AgentMessageRecord> {
    Ok(AgentMessageRecord {
        id: row.get(0)?,
        session_id: row.get(1)?,
        role: row.get(2)?,
        content: row.get(3)?,
        actions: row.get(4)?,
        created_at: row.get(5)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn db() -> Database {
        Database::open_in_memory().expect("in-memory database")
    }

    #[test]
    fn session_crud_and_message_persistence() {
        let db = db();
        let session = db.create_agent_session(Some("My chat"), None).unwrap();
        assert_eq!(session.title, "My chat");
        assert_eq!(session.message_count, 0);

        db.add_agent_message(&session.id, "user", "hello", "[]").unwrap();
        db.add_agent_message(
            &session.id,
            "assistant",
            "hi there",
            r#"[{"type":"run_workflow","payload":{}}]"#,
        )
        .unwrap();
        db.add_agent_message(&session.id, "user", "again", "[]").unwrap();

        // Chronological order regardless of insert batching.
        let msgs = db.list_agent_messages(&session.id, None).unwrap();
        assert_eq!(msgs.len(), 3);
        assert_eq!(msgs[0].role, "user");
        assert_eq!(msgs[0].content, "hello");
        assert_eq!(msgs[1].actions, r#"[{"type":"run_workflow","payload":{}}]"#);
        assert!(msgs[0].id < msgs[1].id && msgs[1].id < msgs[2].id);

        let loaded = db.get_agent_session(&session.id).unwrap().unwrap();
        assert_eq!(loaded.message_count, 3);
        assert_eq!(db.count_agent_messages(&session.id).unwrap(), 3);
    }

    #[test]
    fn recent_window_is_returned_oldest_first() {
        let db = db();
        let s = db.create_agent_session(None, None).unwrap();
        for i in 0..5 {
            db.add_agent_message(&s.id, "user", &format!("m{i}"), "[]")
                .unwrap();
        }
        let window = db.list_agent_messages(&s.id, Some(3)).unwrap();
        assert_eq!(window.len(), 3);
        assert_eq!(window[0].content, "m2");
        assert_eq!(window[2].content, "m4");
    }

    #[test]
    fn deleting_session_cascades_messages() {
        let db = db();
        let s1 = db.create_agent_session(Some("s1"), None).unwrap();
        let s2 = db.create_agent_session(Some("s2"), None).unwrap();
        db.add_agent_message(&s1.id, "user", "a", "[]").unwrap();
        db.add_agent_message(&s2.id, "user", "b", "[]").unwrap();

        db.delete_agent_session(&s1.id).unwrap();

        assert!(db.get_agent_session(&s1.id).unwrap().is_none());
        assert_eq!(db.count_agent_messages(&s1.id).unwrap(), 0);
        // The other session is untouched.
        assert_eq!(db.count_agent_messages(&s2.id).unwrap(), 1);
        assert_eq!(db.list_agent_sessions(None).unwrap().len(), 1);
    }

    #[test]
    fn message_requires_existing_session() {
        let db = db();
        // PRAGMA foreign_keys=ON must reject the orphan insert.
        let err = db
            .add_agent_message("missing-id", "user", "x", "[]")
            .expect_err("foreign key violation expected");
        assert!(matches!(err, DatabaseError::Sqlite(_)));
    }

    #[test]
    fn rename_and_recency_ordering() {
        let db = db();
        let older = db.create_agent_session(Some("older"), None).unwrap();
        let newer = db.create_agent_session(Some("newer"), None).unwrap();
        // Even if both rows share the same second-level timestamp, activity
        // (a message with a higher id) must float `older` above `newer`.
        db.add_agent_message(&older.id, "user", "ping", "[]").unwrap();

        let sessions = db.list_agent_sessions(None).unwrap();
        assert_eq!(sessions[0].id, older.id);
        assert_eq!(sessions[1].id, newer.id);

        assert!(db.rename_agent_session(&older.id, "renamed").unwrap());
        assert_eq!(
            db.get_agent_session(&older.id)
                .unwrap()
                .unwrap()
                .title,
            "renamed"
        );
        assert!(!db.rename_agent_session("nope", "x").unwrap());
    }
}

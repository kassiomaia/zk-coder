//! SQLite storage for conversation context.
//!
//! This module provides persistence for context windows using SQLite.

use std::sync::Mutex;

use chrono::{DateTime, Utc};
use rusqlite::{params, Connection, OptionalExtension};
use thiserror::Error;

use crate::nlp::context::{ContextMessage, ContextMetadata, ContextWindow, MessageRole};

/// SQLite-based context store.
pub struct ContextStore {
    /// Database connection (protected by mutex for thread safety)
    conn: Mutex<Connection>,
}

impl ContextStore {
    /// Creates a new context store at the given path.
    ///
    /// # Errors
    ///
    /// Returns an error if the database cannot be opened or initialized.
    pub fn new(db_path: &str) -> Result<Self, StoreError> {
        let conn = Connection::open(db_path)?;
        Self::init_schema(&conn)?;
        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    /// Creates an in-memory context store (for testing).
    ///
    /// # Errors
    ///
    /// Returns an error if the database cannot be initialized.
    pub fn in_memory() -> Result<Self, StoreError> {
        let conn = Connection::open_in_memory()?;
        Self::init_schema(&conn)?;
        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    fn init_schema(conn: &Connection) -> Result<(), StoreError> {
        conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS context_windows (
                id TEXT PRIMARY KEY,
                conversation_id TEXT NOT NULL UNIQUE,
                thread_id TEXT,
                metadata TEXT NOT NULL,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS context_messages (
                id TEXT PRIMARY KEY,
                context_id TEXT NOT NULL,
                role TEXT NOT NULL,
                content TEXT NOT NULL,
                timestamp TEXT NOT NULL,
                tokens INTEGER,
                FOREIGN KEY (context_id) REFERENCES context_windows(id) ON DELETE CASCADE
            );

            CREATE INDEX IF NOT EXISTS idx_messages_context
            ON context_messages(context_id);

            CREATE INDEX IF NOT EXISTS idx_conversation_id
            ON context_windows(conversation_id);
            "#,
        )?;
        Ok(())
    }

    /// Loads a context window by conversation ID.
    ///
    /// # Errors
    ///
    /// Returns an error if the database query fails or data is invalid.
    pub fn load(&self, conversation_id: &str) -> Result<ContextWindow, StoreError> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| StoreError::LockError("Failed to acquire lock".to_string()))?;

        // Load window
        let window: Option<(String, Option<String>, String, String, String)> = conn
            .query_row(
                "SELECT id, thread_id, metadata, created_at, updated_at
                 FROM context_windows WHERE conversation_id = ?1",
                params![conversation_id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?)),
            )
            .optional()?;

        let (id, thread_id, metadata_json, created_at, updated_at) =
            window.ok_or(StoreError::NotFound(conversation_id.to_string()))?;

        let metadata: ContextMetadata = serde_json::from_str(&metadata_json)?;

        // Load messages
        let mut stmt = conn.prepare(
            "SELECT id, role, content, timestamp, tokens
             FROM context_messages WHERE context_id = ?1
             ORDER BY timestamp ASC",
        )?;

        let messages = stmt
            .query_map(params![&id], |row| {
                Ok(ContextMessage {
                    id: row.get(0)?,
                    role: MessageRole::from_str(&row.get::<_, String>(1)?),
                    content: row.get(2)?,
                    timestamp: parse_datetime(&row.get::<_, String>(3)?),
                    tokens: row.get(4)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(ContextWindow {
            id,
            conversation_id: conversation_id.to_string(),
            thread_id,
            messages,
            metadata,
            created_at: parse_datetime(&created_at),
            updated_at: parse_datetime(&updated_at),
        })
    }

    /// Saves a context window.
    ///
    /// # Errors
    ///
    /// Returns an error if the database operation fails.
    pub fn save(&self, window: &ContextWindow) -> Result<(), StoreError> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| StoreError::LockError("Failed to acquire lock".to_string()))?;

        let metadata_json = serde_json::to_string(&window.metadata)?;

        // Upsert window
        conn.execute(
            "INSERT OR REPLACE INTO context_windows
             (id, conversation_id, thread_id, metadata, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                &window.id,
                &window.conversation_id,
                &window.thread_id,
                &metadata_json,
                &window.created_at.to_rfc3339(),
                &window.updated_at.to_rfc3339(),
            ],
        )?;

        // Delete old messages
        conn.execute(
            "DELETE FROM context_messages WHERE context_id = ?1",
            params![&window.id],
        )?;

        // Insert new messages
        for msg in &window.messages {
            conn.execute(
                "INSERT INTO context_messages (id, context_id, role, content, timestamp, tokens)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![
                    &msg.id,
                    &window.id,
                    msg.role.as_str(),
                    &msg.content,
                    &msg.timestamp.to_rfc3339(),
                    msg.tokens,
                ],
            )?;
        }

        Ok(())
    }

    /// Creates a new context window.
    ///
    /// # Errors
    ///
    /// Returns an error if the database operation fails.
    pub fn create(&self, conversation_id: &str) -> Result<ContextWindow, StoreError> {
        let window = ContextWindow::new(conversation_id);
        self.save(&window)?;
        Ok(window)
    }

    /// Checks if a context window exists.
    ///
    /// # Errors
    ///
    /// Returns an error if the database query fails.
    pub fn exists(&self, conversation_id: &str) -> Result<bool, StoreError> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| StoreError::LockError("Failed to acquire lock".to_string()))?;

        let count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM context_windows WHERE conversation_id = ?1",
            params![conversation_id],
            |row| row.get(0),
        )?;

        Ok(count > 0)
    }

    /// Deletes a context window.
    ///
    /// # Errors
    ///
    /// Returns an error if the database operation fails.
    pub fn delete(&self, conversation_id: &str) -> Result<(), StoreError> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| StoreError::LockError("Failed to acquire lock".to_string()))?;

        conn.execute(
            "DELETE FROM context_windows WHERE conversation_id = ?1",
            params![conversation_id],
        )?;

        Ok(())
    }
}

/// Parses a datetime string, returning Utc::now() on failure.
fn parse_datetime(s: &str) -> DateTime<Utc> {
    chrono::DateTime::parse_from_rfc3339(s)
        .map(|dt| dt.with_timezone(&Utc))
        .unwrap_or_else(|_| Utc::now())
}

/// Errors from the context store.
#[derive(Debug, Error)]
pub enum StoreError {
    /// SQLite error
    #[error("SQLite error: {0}")]
    Sqlite(#[from] rusqlite::Error),
    /// Not found error
    #[error("Context not found: {0}")]
    NotFound(String),
    /// Lock error
    #[error("Lock error: {0}")]
    LockError(String),
    /// JSON error
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_and_load() {
        let store = ContextStore::in_memory().expect("Failed to create store");

        let window = store.create("conv-123").expect("Failed to create");
        assert_eq!(window.conversation_id, "conv-123");
        assert!(window.messages.is_empty());

        let loaded = store.load("conv-123").expect("Failed to load");
        assert_eq!(loaded.id, window.id);
    }

    #[test]
    fn test_save_with_messages() {
        let store = ContextStore::in_memory().expect("Failed to create store");

        let mut window = store.create("conv-456").expect("Failed to create");
        window
            .messages
            .push(ContextMessage::user("Hello world"));

        store.save(&window).expect("Failed to save");

        let loaded = store.load("conv-456").expect("Failed to load");
        assert_eq!(loaded.messages.len(), 1);
        assert_eq!(loaded.messages[0].content, "Hello world");
    }

    #[test]
    fn test_exists() {
        let store = ContextStore::in_memory().expect("Failed to create store");

        assert!(!store.exists("conv-789").expect("Failed to check"));

        store.create("conv-789").expect("Failed to create");

        assert!(store.exists("conv-789").expect("Failed to check"));
    }

    #[test]
    fn test_delete() {
        let store = ContextStore::in_memory().expect("Failed to create store");

        store.create("conv-delete").expect("Failed to create");
        assert!(store.exists("conv-delete").expect("Failed to check"));

        store.delete("conv-delete").expect("Failed to delete");
        assert!(!store.exists("conv-delete").expect("Failed to check"));
    }

    #[test]
    fn test_not_found() {
        let store = ContextStore::in_memory().expect("Failed to create store");

        let result = store.load("nonexistent");
        assert!(result.is_err());
    }
}

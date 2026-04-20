//! `SQLite` repository for conversation context windows.
//!
//! This module provides persistence for NLP context windows using `SQLite`.
//! It stores conversation messages and metadata for context-aware AI interactions.

use std::sync::{Arc, Mutex};

use chrono::{DateTime, Utc};
use rusqlite::{params, Connection, OptionalExtension};
use thiserror::Error;

use crate::core::nlp::entities::{ContextMessage, ContextMetadata, ContextWindow, MessageRole};
use crate::core::nlp::repositories::{ContextWindowRepository, NlpDomainError, RepositoryResult};
use crate::infrastructure::sqlite::migrations::nlp_context::init_schema;

/// SQLite-backed context window repository.
pub struct SqliteContextWindowRepository {
    /// Database connection (protected by mutex for thread safety)
    conn: Arc<Mutex<Connection>>,
}

impl SqliteContextWindowRepository {
    /// Creates a new context window repository at the given path.
    pub fn new(db_path: &str) -> Result<Self, ContextRepositoryError> {
        let conn = Connection::open(db_path)?;
        conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;")?;
        init_schema(&conn)?;
        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    /// Creates a context window repository from a shared connection.
    ///
    /// The schema must already have been applied via `migrations::init_all`.
    pub fn from_shared(conn: Arc<Mutex<Connection>>) -> Self {
        Self { conn }
    }

    /// Creates an in-memory repository for testing.
    pub fn in_memory() -> Result<Self, ContextRepositoryError> {
        let conn = Connection::open_in_memory()?;
        init_schema(&conn)?;
        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    /// Loads a context window by conversation ID.
    ///
    /// # Errors
    ///
    /// Returns an error if the database query fails or data is invalid.
    pub fn load_window(
        &self,
        conversation_id: &str,
    ) -> Result<ContextWindow, ContextRepositoryError> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| ContextRepositoryError::LockError("Failed to acquire lock".to_string()))?;

        // Load window
        let window: Option<(String, Option<String>, String, String, String)> = conn
            .query_row(
                "SELECT id, thread_id, metadata, created_at, updated_at
                 FROM context_windows WHERE conversation_id = ?1",
                params![conversation_id],
                |row| {
                    Ok((
                        row.get(0)?,
                        row.get(1)?,
                        row.get(2)?,
                        row.get(3)?,
                        row.get(4)?,
                    ))
                },
            )
            .optional()?;

        let (id, thread_id, metadata_json, created_at, updated_at) = window.ok_or(
            ContextRepositoryError::NotFound(conversation_id.to_string()),
        )?;

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
                    role: MessageRole::from_str_or_default(&row.get::<_, String>(1)?),
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
    pub fn save_window(&self, window: &ContextWindow) -> Result<(), ContextRepositoryError> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| ContextRepositoryError::LockError("Failed to acquire lock".to_string()))?;

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
    pub fn create_window(
        &self,
        conversation_id: &str,
    ) -> Result<ContextWindow, ContextRepositoryError> {
        let window = ContextWindow::new(conversation_id);
        self.save_window(&window)?;
        Ok(window)
    }

    /// Checks if a context window exists.
    ///
    /// # Errors
    ///
    /// Returns an error if the database query fails.
    pub fn window_exists(&self, conversation_id: &str) -> Result<bool, ContextRepositoryError> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| ContextRepositoryError::LockError("Failed to acquire lock".to_string()))?;

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
    pub fn delete_window(&self, conversation_id: &str) -> Result<(), ContextRepositoryError> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| ContextRepositoryError::LockError("Failed to acquire lock".to_string()))?;

        conn.execute(
            "DELETE FROM context_windows WHERE conversation_id = ?1",
            params![conversation_id],
        )?;

        Ok(())
    }
}

/// Parses a datetime string, returning `Utc::now()` on failure.
fn parse_datetime(s: &str) -> DateTime<Utc> {
    chrono::DateTime::parse_from_rfc3339(s).map_or_else(|_| Utc::now(), |dt| dt.with_timezone(&Utc))
}

/// Errors from the context repository.
#[derive(Debug, Error)]
pub enum ContextRepositoryError {
    /// `SQLite` database error.
    #[error("SQLite error: {0}")]
    Sqlite(#[from] rusqlite::Error),
    /// Context window not found for the given conversation ID.
    #[error("Context not found: {0}")]
    NotFound(String),
    /// Failed to acquire database lock.
    #[error("Lock error: {0}")]
    LockError(String),
    /// JSON serialization/deserialization error.
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
}

impl From<ContextRepositoryError> for NlpDomainError {
    fn from(err: ContextRepositoryError) -> Self {
        match err {
            ContextRepositoryError::NotFound(conversation_id) => {
                Self::ContextNotFound { conversation_id }
            }
            ContextRepositoryError::Sqlite(e) => Self::InvalidInput {
                reason: format!("Database error: {e}"),
            },
            ContextRepositoryError::LockError(reason) => Self::InvalidInput { reason },
            ContextRepositoryError::Json(e) => Self::InvalidInput {
                reason: format!("JSON error: {e}"),
            },
        }
    }
}

impl ContextWindowRepository for SqliteContextWindowRepository {
    fn load(&self, conversation_id: &str) -> RepositoryResult<ContextWindow> {
        self.load_window(conversation_id).map_err(Into::into)
    }

    fn save(&self, window: &ContextWindow) -> RepositoryResult<()> {
        self.save_window(window).map_err(Into::into)
    }

    fn create(&self, conversation_id: &str) -> RepositoryResult<ContextWindow> {
        self.create_window(conversation_id).map_err(Into::into)
    }

    fn exists(&self, conversation_id: &str) -> RepositoryResult<bool> {
        self.window_exists(conversation_id).map_err(Into::into)
    }

    fn delete(&self, conversation_id: &str) -> RepositoryResult<()> {
        self.delete_window(conversation_id).map_err(Into::into)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_and_load() {
        let store =
            SqliteContextWindowRepository::in_memory().expect("Failed to create repository");

        let window = store.create_window("conv-123").expect("Failed to create");
        assert_eq!(window.conversation_id, "conv-123");
        assert!(window.messages.is_empty());

        let loaded = store.load_window("conv-123").expect("Failed to load");
        assert_eq!(loaded.id, window.id);
    }

    #[test]
    fn test_save_with_messages() {
        let store =
            SqliteContextWindowRepository::in_memory().expect("Failed to create repository");

        let mut window = store.create_window("conv-456").expect("Failed to create");
        window.messages.push(ContextMessage::user("Hello world"));

        store.save_window(&window).expect("Failed to save");

        let loaded = store.load_window("conv-456").expect("Failed to load");
        assert_eq!(loaded.messages.len(), 1);
        assert_eq!(loaded.messages[0].content, "Hello world");
    }

    #[test]
    fn test_exists() {
        let store =
            SqliteContextWindowRepository::in_memory().expect("Failed to create repository");

        assert!(!store.window_exists("conv-789").expect("Failed to check"));

        store.create_window("conv-789").expect("Failed to create");

        assert!(store.window_exists("conv-789").expect("Failed to check"));
    }

    #[test]
    fn test_delete() {
        let store =
            SqliteContextWindowRepository::in_memory().expect("Failed to create repository");

        store
            .create_window("conv-delete")
            .expect("Failed to create");
        assert!(store.window_exists("conv-delete").expect("Failed to check"));

        store
            .delete_window("conv-delete")
            .expect("Failed to delete");
        assert!(!store.window_exists("conv-delete").expect("Failed to check"));
    }

    #[test]
    fn test_not_found() {
        let store =
            SqliteContextWindowRepository::in_memory().expect("Failed to create repository");

        let result = store.load_window("nonexistent");
        assert!(result.is_err());
    }
}

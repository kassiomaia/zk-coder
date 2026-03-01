//! SQLite storage for chat conversations and threads.
//!
//! This module provides persistence for conversations, threads, and messages
//! using SQLite.

use std::sync::Mutex;

use chrono::{DateTime, Utc};
use rusqlite::{params, Connection, OptionalExtension};
use thiserror::Error;

use crate::nlp::chat::{
    ChatMessage, Conversation, ConversationSettings, MessageRole, MessageStatus,
    Thread, ThreadMetadata,
};

/// SQLite-based chat store.
pub struct ChatStore {
    /// Database connection (protected by mutex for thread safety)
    conn: Mutex<Connection>,
}

/// Errors from the chat store.
#[derive(Debug, Error)]
pub enum ChatStoreError {
    /// SQLite error
    #[error("SQLite error: {0}")]
    Sqlite(#[from] rusqlite::Error),
    /// Not found error
    #[error("{0} not found: {1}")]
    NotFound(&'static str, String),
    /// Lock error
    #[error("Lock error: {0}")]
    LockError(String),
    /// JSON error
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
}

impl ChatStore {
    /// Creates a new chat store at the given path.
    ///
    /// # Errors
    ///
    /// Returns an error if the database cannot be opened or initialized.
    pub fn new(db_path: &str) -> Result<Self, ChatStoreError> {
        let conn = Connection::open(db_path)?;
        Self::init_schema(&conn)?;
        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    /// Creates an in-memory chat store (for testing).
    ///
    /// # Errors
    ///
    /// Returns an error if the database cannot be initialized.
    pub fn in_memory() -> Result<Self, ChatStoreError> {
        let conn = Connection::open_in_memory()?;
        Self::init_schema(&conn)?;
        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    fn init_schema(conn: &Connection) -> Result<(), ChatStoreError> {
        conn.execute_batch(
            r#"
            -- Conversations table
            CREATE TABLE IF NOT EXISTS conversations (
                id TEXT PRIMARY KEY,
                title TEXT NOT NULL,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                settings TEXT NOT NULL
            );

            CREATE INDEX IF NOT EXISTS idx_conversations_updated
            ON conversations(updated_at DESC);

            -- Threads table
            CREATE TABLE IF NOT EXISTS threads (
                id TEXT PRIMARY KEY,
                conversation_id TEXT NOT NULL,
                parent_id TEXT,
                title TEXT NOT NULL,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                message_count INTEGER NOT NULL DEFAULT 0,
                metadata TEXT NOT NULL,
                FOREIGN KEY (conversation_id) REFERENCES conversations(id) ON DELETE CASCADE,
                FOREIGN KEY (parent_id) REFERENCES threads(id) ON DELETE SET NULL
            );

            CREATE INDEX IF NOT EXISTS idx_threads_conversation
            ON threads(conversation_id);

            CREATE INDEX IF NOT EXISTS idx_threads_updated
            ON threads(updated_at DESC);

            -- Messages table
            CREATE TABLE IF NOT EXISTS chat_messages (
                id TEXT PRIMARY KEY,
                conversation_id TEXT NOT NULL,
                thread_id TEXT,
                role TEXT NOT NULL,
                content TEXT NOT NULL,
                timestamp TEXT NOT NULL,
                status TEXT NOT NULL DEFAULT 'sent',
                metadata TEXT,
                FOREIGN KEY (conversation_id) REFERENCES conversations(id) ON DELETE CASCADE,
                FOREIGN KEY (thread_id) REFERENCES threads(id) ON DELETE SET NULL
            );

            CREATE INDEX IF NOT EXISTS idx_messages_conversation
            ON chat_messages(conversation_id, timestamp ASC);

            CREATE INDEX IF NOT EXISTS idx_messages_thread
            ON chat_messages(thread_id, timestamp ASC);
            "#,
        )?;
        Ok(())
    }

    // =========================================================================
    // Conversation Operations
    // =========================================================================

    /// Creates a new conversation.
    ///
    /// # Errors
    ///
    /// Returns an error if the database operation fails.
    pub fn create_conversation(&self, title: &str) -> Result<Conversation, ChatStoreError> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| ChatStoreError::LockError("Failed to acquire lock".to_string()))?;

        let conversation = Conversation::new(title);
        let settings_json = serde_json::to_string(&conversation.settings)?;

        conn.execute(
            "INSERT INTO conversations (id, title, created_at, updated_at, settings)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                &conversation.id,
                &conversation.title,
                &conversation.created_at.to_rfc3339(),
                &conversation.updated_at.to_rfc3339(),
                &settings_json,
            ],
        )?;

        Ok(conversation)
    }

    /// Loads a conversation by ID.
    ///
    /// # Errors
    ///
    /// Returns an error if the conversation is not found or database fails.
    pub fn load_conversation(&self, id: &str) -> Result<Conversation, ChatStoreError> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| ChatStoreError::LockError("Failed to acquire lock".to_string()))?;

        let row = conn
            .query_row(
                "SELECT id, title, created_at, updated_at, settings
                 FROM conversations WHERE id = ?1",
                params![id],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, String>(3)?,
                        row.get::<_, String>(4)?,
                    ))
                },
            )
            .optional()?;

        let (id, title, created_at, updated_at, settings_json) =
            row.ok_or(ChatStoreError::NotFound("Conversation", id.to_string()))?;

        let settings: ConversationSettings = serde_json::from_str(&settings_json)?;

        Ok(Conversation {
            id,
            title,
            created_at: parse_datetime(&created_at),
            updated_at: parse_datetime(&updated_at),
            settings,
        })
    }

    /// Loads all conversations, sorted by update time (newest first).
    ///
    /// # Errors
    ///
    /// Returns an error if the database query fails.
    pub fn load_all_conversations(&self) -> Result<Vec<Conversation>, ChatStoreError> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| ChatStoreError::LockError("Failed to acquire lock".to_string()))?;

        let mut stmt = conn.prepare(
            "SELECT id, title, created_at, updated_at, settings
             FROM conversations ORDER BY updated_at DESC",
        )?;

        let conversations = stmt
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                ))
            })?
            .map(|result| {
                let (id, title, created_at, updated_at, settings_json) = result?;
                let settings: ConversationSettings = serde_json::from_str(&settings_json)?;
                Ok(Conversation {
                    id,
                    title,
                    created_at: parse_datetime(&created_at),
                    updated_at: parse_datetime(&updated_at),
                    settings,
                })
            })
            .collect::<Result<Vec<_>, ChatStoreError>>()?;

        Ok(conversations)
    }

    /// Updates a conversation's title.
    ///
    /// # Errors
    ///
    /// Returns an error if the conversation is not found or database fails.
    pub fn update_conversation_title(
        &self,
        id: &str,
        title: &str,
    ) -> Result<Conversation, ChatStoreError> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| ChatStoreError::LockError("Failed to acquire lock".to_string()))?;

        let now = Utc::now();
        let rows_affected = conn.execute(
            "UPDATE conversations SET title = ?1, updated_at = ?2 WHERE id = ?3",
            params![title, now.to_rfc3339(), id],
        )?;

        if rows_affected == 0 {
            return Err(ChatStoreError::NotFound("Conversation", id.to_string()));
        }

        drop(conn);
        self.load_conversation(id)
    }

    /// Deletes a conversation and all its threads and messages.
    ///
    /// # Errors
    ///
    /// Returns an error if the database operation fails.
    pub fn delete_conversation(&self, id: &str) -> Result<(), ChatStoreError> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| ChatStoreError::LockError("Failed to acquire lock".to_string()))?;

        conn.execute("DELETE FROM conversations WHERE id = ?1", params![id])?;

        Ok(())
    }

    // =========================================================================
    // Thread Operations
    // =========================================================================

    /// Creates a new thread in a conversation.
    ///
    /// # Errors
    ///
    /// Returns an error if the database operation fails.
    pub fn create_thread(
        &self,
        conversation_id: &str,
        title: &str,
    ) -> Result<Thread, ChatStoreError> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| ChatStoreError::LockError("Failed to acquire lock".to_string()))?;

        let thread = Thread::new(conversation_id, title);
        let metadata_json = serde_json::to_string(&thread.metadata)?;

        conn.execute(
            "INSERT INTO threads (id, conversation_id, parent_id, title, created_at, updated_at, message_count, metadata)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                &thread.id,
                &thread.conversation_id,
                &thread.parent_id,
                &thread.title,
                &thread.created_at.to_rfc3339(),
                &thread.updated_at.to_rfc3339(),
                thread.message_count as i64,
                &metadata_json,
            ],
        )?;

        Ok(thread)
    }

    /// Loads all threads for a conversation.
    ///
    /// # Errors
    ///
    /// Returns an error if the database query fails.
    pub fn load_threads(&self, conversation_id: &str) -> Result<Vec<Thread>, ChatStoreError> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| ChatStoreError::LockError("Failed to acquire lock".to_string()))?;

        let mut stmt = conn.prepare(
            "SELECT id, conversation_id, parent_id, title, created_at, updated_at, message_count, metadata
             FROM threads WHERE conversation_id = ?1 ORDER BY updated_at DESC",
        )?;

        let threads = stmt
            .query_map(params![conversation_id], |row| {
                Ok(Thread {
                    id: row.get(0)?,
                    conversation_id: row.get(1)?,
                    parent_id: row.get(2)?,
                    title: row.get(3)?,
                    created_at: parse_datetime(&row.get::<_, String>(4)?),
                    updated_at: parse_datetime(&row.get::<_, String>(5)?),
                    message_count: row.get::<_, i64>(6)? as usize,
                    metadata: serde_json::from_str(&row.get::<_, String>(7)?).unwrap_or_default(),
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(threads)
    }

    /// Updates a thread's resolved status.
    ///
    /// # Errors
    ///
    /// Returns an error if the database operation fails.
    pub fn update_thread_resolved(
        &self,
        id: &str,
        resolved: bool,
    ) -> Result<(), ChatStoreError> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| ChatStoreError::LockError("Failed to acquire lock".to_string()))?;

        // Load current metadata
        let metadata_json: Option<String> = conn
            .query_row(
                "SELECT metadata FROM threads WHERE id = ?1",
                params![id],
                |row| row.get(0),
            )
            .optional()?;

        let mut metadata: ThreadMetadata = metadata_json
            .and_then(|j| serde_json::from_str(&j).ok())
            .unwrap_or_default();
        metadata.resolved = resolved;

        let new_metadata_json = serde_json::to_string(&metadata)?;
        let now = Utc::now();

        conn.execute(
            "UPDATE threads SET metadata = ?1, updated_at = ?2 WHERE id = ?3",
            params![new_metadata_json, now.to_rfc3339(), id],
        )?;

        Ok(())
    }

    /// Deletes a thread and all its messages.
    ///
    /// # Errors
    ///
    /// Returns an error if the database operation fails.
    pub fn delete_thread(&self, id: &str) -> Result<(), ChatStoreError> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| ChatStoreError::LockError("Failed to acquire lock".to_string()))?;

        conn.execute("DELETE FROM threads WHERE id = ?1", params![id])?;

        Ok(())
    }

    // =========================================================================
    // Message Operations
    // =========================================================================

    /// Adds a message to a conversation (optionally to a thread).
    ///
    /// # Errors
    ///
    /// Returns an error if the database operation fails.
    pub fn add_message(&self, message: &ChatMessage) -> Result<(), ChatStoreError> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| ChatStoreError::LockError("Failed to acquire lock".to_string()))?;

        let metadata_json = message
            .metadata
            .as_ref()
            .map(|m| serde_json::to_string(m))
            .transpose()?;

        conn.execute(
            "INSERT INTO chat_messages (id, conversation_id, thread_id, role, content, timestamp, status, metadata)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                &message.id,
                &message.conversation_id,
                &message.thread_id,
                message.role.as_str(),
                &message.content,
                &message.timestamp.to_rfc3339(),
                message.status.as_str(),
                metadata_json,
            ],
        )?;

        // Update conversation's updated_at
        let now = Utc::now();
        conn.execute(
            "UPDATE conversations SET updated_at = ?1 WHERE id = ?2",
            params![now.to_rfc3339(), &message.conversation_id],
        )?;

        // Update thread's message count and updated_at if applicable
        if let Some(ref thread_id) = message.thread_id {
            conn.execute(
                "UPDATE threads SET message_count = message_count + 1, updated_at = ?1 WHERE id = ?2",
                params![now.to_rfc3339(), thread_id],
            )?;
        }

        Ok(())
    }

    /// Loads messages for a conversation.
    ///
    /// # Errors
    ///
    /// Returns an error if the database query fails.
    pub fn load_messages(&self, conversation_id: &str) -> Result<Vec<ChatMessage>, ChatStoreError> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| ChatStoreError::LockError("Failed to acquire lock".to_string()))?;

        let mut stmt = conn.prepare(
            "SELECT id, conversation_id, thread_id, role, content, timestamp, status, metadata
             FROM chat_messages WHERE conversation_id = ?1 ORDER BY timestamp ASC",
        )?;

        let messages = stmt
            .query_map(params![conversation_id], |row| {
                let metadata_json: Option<String> = row.get(7)?;
                let metadata = metadata_json
                    .and_then(|j| serde_json::from_str(&j).ok());

                Ok(ChatMessage {
                    id: row.get(0)?,
                    conversation_id: row.get(1)?,
                    thread_id: row.get(2)?,
                    role: MessageRole::from_str(&row.get::<_, String>(3)?),
                    content: row.get(4)?,
                    timestamp: parse_datetime(&row.get::<_, String>(5)?),
                    status: parse_status(&row.get::<_, String>(6)?),
                    metadata,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(messages)
    }

    /// Loads messages for a specific thread.
    ///
    /// # Errors
    ///
    /// Returns an error if the database query fails.
    pub fn load_thread_messages(&self, thread_id: &str) -> Result<Vec<ChatMessage>, ChatStoreError> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| ChatStoreError::LockError("Failed to acquire lock".to_string()))?;

        let mut stmt = conn.prepare(
            "SELECT id, conversation_id, thread_id, role, content, timestamp, status, metadata
             FROM chat_messages WHERE thread_id = ?1 ORDER BY timestamp ASC",
        )?;

        let messages = stmt
            .query_map(params![thread_id], |row| {
                let metadata_json: Option<String> = row.get(7)?;
                let metadata = metadata_json
                    .and_then(|j| serde_json::from_str(&j).ok());

                Ok(ChatMessage {
                    id: row.get(0)?,
                    conversation_id: row.get(1)?,
                    thread_id: row.get(2)?,
                    role: MessageRole::from_str(&row.get::<_, String>(3)?),
                    content: row.get(4)?,
                    timestamp: parse_datetime(&row.get::<_, String>(5)?),
                    status: parse_status(&row.get::<_, String>(6)?),
                    metadata,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(messages)
    }

    /// Deletes a message.
    ///
    /// # Errors
    ///
    /// Returns an error if the database operation fails.
    pub fn delete_message(&self, id: &str) -> Result<(), ChatStoreError> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| ChatStoreError::LockError("Failed to acquire lock".to_string()))?;

        conn.execute(
            "DELETE FROM chat_messages WHERE id = ?1",
            params![id],
        )?;

        Ok(())
    }

    /// Updates a message's status.
    ///
    /// # Errors
    ///
    /// Returns an error if the database operation fails.
    pub fn update_message_status(
        &self,
        id: &str,
        status: MessageStatus,
    ) -> Result<(), ChatStoreError> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| ChatStoreError::LockError("Failed to acquire lock".to_string()))?;

        conn.execute(
            "UPDATE chat_messages SET status = ?1 WHERE id = ?2",
            params![status.as_str(), id],
        )?;

        Ok(())
    }

    // =========================================================================
    // Migration Operations
    // =========================================================================

    /// Migrates orphan messages (messages without a thread) to new threads.
    ///
    /// For each conversation that has messages with NULL thread_id:
    /// 1. Creates a new thread using the first message content as the title
    /// 2. Updates all orphan messages in that conversation to reference the new thread
    ///
    /// This migration is idempotent (safe to run multiple times).
    ///
    /// # Errors
    ///
    /// Returns an error if the database operation fails.
    pub fn migrate_orphan_messages_to_threads(&self) -> Result<(), ChatStoreError> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| ChatStoreError::LockError("Failed to acquire lock".to_string()))?;

        // Find conversations with orphan messages
        let orphan_conversations: Vec<(String, String, String)> = conn
            .prepare(
                "SELECT DISTINCT m.conversation_id, m.id, m.content
                 FROM chat_messages m
                 WHERE m.thread_id IS NULL
                 ORDER BY m.timestamp ASC",
            )?
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))?
            .collect::<Result<Vec<_>, _>>()?;

        // Group by conversation_id to get the first message per conversation
        let mut first_messages: std::collections::HashMap<String, (String, String)> =
            std::collections::HashMap::new();
        for (conv_id, msg_id, content) in orphan_conversations {
            if !first_messages.contains_key(&conv_id) {
                first_messages.insert(conv_id.clone(), (msg_id, content));
            }
        }

        // For each conversation with orphan messages, create a thread and update messages
        let now = Utc::now();
        for (conv_id, (_first_msg_id, first_msg_content)) in first_messages {
            // Derive thread title from first message content (truncated to ~50 chars)
            let thread_title = Self::derive_thread_title(&first_msg_content);

            // Create thread
            let thread_id = uuid::Uuid::new_v4().to_string();
            let metadata_json = serde_json::to_string(&ThreadMetadata::default())?;

            conn.execute(
                "INSERT INTO threads (id, conversation_id, parent_id, title, created_at, updated_at, message_count, metadata)
                 VALUES (?1, ?2, NULL, ?3, ?4, ?5, 0, ?6)",
                params![
                    &thread_id,
                    &conv_id,
                    &thread_title,
                    now.to_rfc3339(),
                    now.to_rfc3339(),
                    &metadata_json,
                ],
            )?;

            // Update orphan messages to reference the new thread
            conn.execute(
                "UPDATE chat_messages SET thread_id = ?1 WHERE conversation_id = ?2 AND thread_id IS NULL",
                params![&thread_id, &conv_id],
            )?;

            // Update thread message count
            let count: i64 = conn.query_row(
                "SELECT COUNT(*) FROM chat_messages WHERE thread_id = ?1",
                params![&thread_id],
                |row| row.get(0),
            )?;

            conn.execute(
                "UPDATE threads SET message_count = ?1 WHERE id = ?2",
                params![count, &thread_id],
            )?;
        }

        Ok(())
    }

    /// Derives a thread title from message content.
    fn derive_thread_title(content: &str) -> String {
        let cleaned = content
            .replace('#', "")
            .replace('*', "")
            .replace('`', "")
            .replace('\n', " ")
            .trim()
            .to_string();

        if cleaned.is_empty() {
            return "New Thread".to_string();
        }

        if cleaned.len() <= 50 {
            return cleaned;
        }

        let truncated = &cleaned[..50];
        if let Some(last_space) = truncated.rfind(' ') {
            if last_space > 20 {
                format!("{}...", &truncated[..last_space])
            } else {
                format!("{}...", truncated)
            }
        } else {
            format!("{}...", truncated)
        }
    }
}

// =========================================================================
// Helper Functions
// =========================================================================

/// Parses a datetime string, returning Utc::now() on failure.
fn parse_datetime(s: &str) -> DateTime<Utc> {
    chrono::DateTime::parse_from_rfc3339(s)
        .map(|dt| dt.with_timezone(&Utc))
        .unwrap_or_else(|_| Utc::now())
}

impl MessageStatus {
    /// Converts status to string for storage.
    fn as_str(&self) -> &'static str {
        match self {
            Self::Sending => "sending",
            Self::Sent => "sent",
            Self::Delivered => "delivered",
            Self::Error => "error",
        }
    }
}

/// Parses status from string.
fn parse_status(s: &str) -> MessageStatus {
    match s {
        "sending" => MessageStatus::Sending,
        "delivered" => MessageStatus::Delivered,
        "error" => MessageStatus::Error,
        _ => MessageStatus::Sent,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_and_load_conversation() {
        let store = ChatStore::in_memory().expect("Failed to create store");

        let conv = store
            .create_conversation("Test Conversation")
            .expect("Failed to create");

        assert_eq!(conv.title, "Test Conversation");

        let loaded = store
            .load_conversation(&conv.id)
            .expect("Failed to load");

        assert_eq!(loaded.id, conv.id);
        assert_eq!(loaded.title, conv.title);
    }

    #[test]
    fn test_create_and_load_thread() {
        let store = ChatStore::in_memory().expect("Failed to create store");

        let conv = store
            .create_conversation("Test")
            .expect("Failed to create");

        let thread = store
            .create_thread(&conv.id, "Test Thread")
            .expect("Failed to create thread");

        assert_eq!(thread.conversation_id, conv.id);
        assert_eq!(thread.title, "Test Thread");

        let threads = store.load_threads(&conv.id).expect("Failed to load");
        assert_eq!(threads.len(), 1);
    }

    #[test]
    fn test_add_and_load_messages() {
        let store = ChatStore::in_memory().expect("Failed to create store");

        let conv = store
            .create_conversation("Test")
            .expect("Failed to create");

        let msg = ChatMessage::user(&conv.id, "Hello world");
        store.add_message(&msg).expect("Failed to add message");

        let messages = store.load_messages(&conv.id).expect("Failed to load");
        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0].content, "Hello world");
        assert_eq!(messages[0].role, MessageRole::User);
    }

    #[test]
    fn test_update_conversation_title() {
        let store = ChatStore::in_memory().expect("Failed to create store");

        let conv = store
            .create_conversation("Old Title")
            .expect("Failed to create");

        let updated = store
            .update_conversation_title(&conv.id, "New Title")
            .expect("Failed to update");

        assert_eq!(updated.title, "New Title");
    }

    #[test]
    fn test_delete_conversation() {
        let store = ChatStore::in_memory().expect("Failed to create store");

        let conv = store
            .create_conversation("Test")
            .expect("Failed to create");

        store
            .delete_conversation(&conv.id)
            .expect("Failed to delete");

        let result = store.load_conversation(&conv.id);
        assert!(result.is_err());
    }
}

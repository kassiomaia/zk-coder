//! SQLite storage for chat conversations and threads.
//!
//! This module provides persistence for conversations, threads, and messages
//! using SQLite.

mod schema;
mod conversations;
mod threads;
mod messages;
mod migrations;
mod helpers;

use std::sync::Mutex;

use rusqlite::Connection;
use thiserror::Error;

pub use schema::init_schema;

pub use conversations::{
    create_conversation,
    load_conversation,
    load_all_conversations,
    update_conversation_title,
    delete_conversation,
    update_conversation_settings,
    update_conversation_workspace,
};

pub use threads::{
    create_thread,
    load_threads,
    load_thread,
    update_thread_resolved,
    set_thread_agent_session,
    delete_thread,
    rename_thread,
    create_thread_with_message,
};

pub use messages::{
    add_message,
    load_messages,
    load_thread_messages,
    delete_message,
    update_message_status,
    update_message_feedback,
    load_thread_feedback,
};

pub use migrations::migrate_orphan_messages_to_threads;

use crate::nlp::chat::{
    ChatMessage, Conversation, ConversationSettings, MessageFeedback, MessageMetadata,
    MessageRole, Thread, ThreadMetadata,
};

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
    /// Thread is resolved and cannot accept new messages
    #[error("Thread '{0}' is resolved and cannot accept new messages. Reopen the thread to continue the conversation.")]
    ThreadResolved(String),
}

/// SQLite-based chat store.
pub struct ChatStore {
    /// Database connection (protected by mutex for thread safety)
    conn: Mutex<Connection>,
}

impl ChatStore {
    /// Creates a new chat store at the given path.
    ///
    /// # Errors
    ///
    /// Returns an error if the database cannot be opened or initialized.
    pub fn new(db_path: &str) -> Result<Self, ChatStoreError> {
        let conn = Connection::open(db_path)?;
        init_schema(&conn)?;

        let store = Self {
            conn: Mutex::new(conn),
        };

        // Run migration for orphan messages
        store.migrate_orphan_messages_to_threads()?;

        Ok(store)
    }

    /// Creates an in-memory chat store (for testing).
    ///
    /// # Errors
    ///
    /// Returns an error if the database cannot be initialized.
    pub fn in_memory() -> Result<Self, ChatStoreError> {
        let conn = Connection::open_in_memory()?;
        init_schema(&conn)?;
        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    // ========================================================================
    // Conversation Operations
    // ========================================================================

    /// Creates a new conversation.
    pub fn create_conversation(&self, title: &str) -> Result<Conversation, ChatStoreError> {
        let conn = self.acquire_lock()?;
        conversations::create_conversation(&conn, title)
    }

    /// Loads a conversation by ID.
    pub fn load_conversation(&self, id: &str) -> Result<Conversation, ChatStoreError> {
        let conn = self.acquire_lock()?;
        conversations::load_conversation(&conn, id)
    }

    /// Loads all conversations, sorted by update time (newest first).
    pub fn load_all_conversations(&self) -> Result<Vec<Conversation>, ChatStoreError> {
        let conn = self.acquire_lock()?;
        conversations::load_all_conversations(&conn)
    }

    /// Updates a conversation's title.
    pub fn update_conversation_title(
        &self,
        id: &str,
        title: &str,
    ) -> Result<Conversation, ChatStoreError> {
        let conn = self.acquire_lock()?;
        conversations::update_conversation_title(&conn, id, title)
    }

    /// Deletes a conversation and all its threads and messages.
    pub fn delete_conversation(&self, id: &str) -> Result<(), ChatStoreError> {
        let conn = self.acquire_lock()?;
        conversations::delete_conversation(&conn, id)
    }

    /// Updates a conversation's settings.
    pub fn update_conversation_settings(
        &self,
        id: &str,
        settings: &ConversationSettings,
    ) -> Result<Conversation, ChatStoreError> {
        let conn = self.acquire_lock()?;
        conversations::update_conversation_settings(&conn, id, settings)
    }

    /// Updates a conversation's workspace root.
    pub fn update_conversation_workspace(
        &self,
        id: &str,
        workspace_root: &str,
    ) -> Result<Conversation, ChatStoreError> {
        let conn = self.acquire_lock()?;
        conversations::update_conversation_workspace(&conn, id, workspace_root)
    }

    // ========================================================================
    // Thread Operations
    // ========================================================================

    /// Creates a new thread in a conversation.
    pub fn create_thread(
        &self,
        conversation_id: &str,
        title: &str,
    ) -> Result<Thread, ChatStoreError> {
        let conn = self.acquire_lock()?;
        threads::create_thread(&conn, conversation_id, title)
    }

    /// Loads all threads for a conversation.
    pub fn load_threads(&self, conversation_id: &str) -> Result<Vec<Thread>, ChatStoreError> {
        let conn = self.acquire_lock()?;
        threads::load_threads(&conn, conversation_id)
    }

    /// Loads a single thread by ID.
    pub fn load_thread(&self, id: &str) -> Result<Thread, ChatStoreError> {
        let conn = self.acquire_lock()?;
        threads::load_thread(&conn, id)
    }

    /// Updates a thread's resolved status.
    pub fn update_thread_resolved(&self, id: &str, resolved: bool) -> Result<(), ChatStoreError> {
        let conn = self.acquire_lock()?;
        threads::update_thread_resolved(&conn, id, resolved)
    }

    /// Sets the agent session ID for a thread.
    pub fn set_thread_agent_session(
        &self,
        id: &str,
        agent_session_id: Option<String>,
    ) -> Result<(), ChatStoreError> {
        let conn = self.acquire_lock()?;
        threads::set_thread_agent_session(&conn, id, agent_session_id)
    }

    /// Deletes a thread and all its messages.
    pub fn delete_thread(&self, id: &str) -> Result<(), ChatStoreError> {
        let conn = self.acquire_lock()?;
        threads::delete_thread(&conn, id)
    }

    /// Renames a thread.
    pub fn rename_thread(&self, id: &str, title: &str) -> Result<Thread, ChatStoreError> {
        let conn = self.acquire_lock()?;
        threads::rename_thread(&conn, id, title)
    }

    /// Creates a thread with its first message atomically.
    pub fn create_thread_with_message(
        &self,
        conversation_id: &str,
        thread_title: &str,
        message_content: &str,
        message_role: MessageRole,
    ) -> Result<(Thread, ChatMessage), ChatStoreError> {
        let conn = self.acquire_lock()?;
        threads::create_thread_with_message(
            &conn,
            conversation_id,
            thread_title,
            message_content,
            message_role,
        )
    }

    // ========================================================================
    // Message Operations
    // ========================================================================

    /// Adds a message to a conversation (optionally to a thread).
    pub fn add_message(&self, message: &ChatMessage) -> Result<(), ChatStoreError> {
        let conn = self.acquire_lock()?;
        messages::add_message(&conn, message)
    }

    /// Loads messages for a conversation.
    pub fn load_messages(&self, conversation_id: &str) -> Result<Vec<ChatMessage>, ChatStoreError> {
        let conn = self.acquire_lock()?;
        messages::load_messages(&conn, conversation_id)
    }

    /// Loads messages for a specific thread.
    pub fn load_thread_messages(&self, thread_id: &str) -> Result<Vec<ChatMessage>, ChatStoreError> {
        let conn = self.acquire_lock()?;
        messages::load_thread_messages(&conn, thread_id)
    }

    /// Deletes a message.
    pub fn delete_message(&self, id: &str) -> Result<(), ChatStoreError> {
        let conn = self.acquire_lock()?;
        messages::delete_message(&conn, id)
    }

    /// Updates a message's status.
    pub fn update_message_status(
        &self,
        id: &str,
        status: crate::nlp::chat::MessageStatus,
    ) -> Result<(), ChatStoreError> {
        let conn = self.acquire_lock()?;
        messages::update_message_status(&conn, id, status)
    }

    /// Updates a message's feedback (like/dislike).
    pub fn update_message_feedback(
        &self,
        id: &str,
        feedback: Option<MessageFeedback>,
    ) -> Result<(), ChatStoreError> {
        let conn = self.acquire_lock()?;
        messages::update_message_feedback(&conn, id, feedback)
    }

    /// Loads messages with feedback for a specific thread.
    pub fn load_thread_feedback(
        &self,
        thread_id: &str,
        limit: usize,
    ) -> Result<Vec<(ChatMessage, MessageFeedback)>, ChatStoreError> {
        let conn = self.acquire_lock()?;
        messages::load_thread_feedback(&conn, thread_id, limit)
    }

    // ========================================================================
    // Migration Operations
    // ========================================================================

    /// Migrates orphan messages to new threads.
    pub fn migrate_orphan_messages_to_threads(&self) -> Result<(), ChatStoreError> {
        let conn = self.acquire_lock()?;
        migrations::migrate_orphan_messages_to_threads(&conn)
    }

    /// Acquires the database lock, returning a formatted error on failure.
    fn acquire_lock(&self) -> Result<std::sync::MutexGuard<'_, Connection>, ChatStoreError> {
        self.conn
            .lock()
            .map_err(|_| ChatStoreError::LockError("Failed to acquire lock".to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_and_load_conversation() {
        let store = ChatStore::in_memory().expect("Failed to create store");
        let conv = store.create_conversation("Test Conversation").expect("Failed to create");
        assert_eq!(conv.title, "Test Conversation");

        let loaded = store.load_conversation(&conv.id).expect("Failed to load");
        assert_eq!(loaded.id, conv.id);
        assert_eq!(loaded.title, conv.title);
    }

    #[test]
    fn test_create_and_load_thread() {
        let store = ChatStore::in_memory().expect("Failed to create store");
        let conv = store.create_conversation("Test").expect("Failed to create");
        let thread = store.create_thread(&conv.id, "Test Thread").expect("Failed to create thread");
        assert_eq!(thread.conversation_id, conv.id);

        let threads = store.load_threads(&conv.id).expect("Failed to load");
        assert_eq!(threads.len(), 1);
    }

    #[test]
    fn test_add_and_load_messages() {
        let store = ChatStore::in_memory().expect("Failed to create store");
        let conv = store.create_conversation("Test").expect("Failed to create");
        let msg = ChatMessage::user(&conv.id, "Hello world");
        store.add_message(&msg).expect("Failed to add message");

        let messages = store.load_messages(&conv.id).expect("Failed to load");
        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0].content, "Hello world");
    }

    #[test]
    fn test_thread_resolved_error_message() {
        let error = ChatStoreError::ThreadResolved("thread-123".to_string());
        assert!(error.to_string().contains("resolved"));
        assert!(error.to_string().contains("thread-123"));
    }

    #[test]
    fn test_conversation_settings_persistence() {
        let store = ChatStore::in_memory().expect("Failed to create store");
        let conv = store.create_conversation("Test").expect("Failed to create");

        let new_settings = ConversationSettings {
            model: "custom-model".to_string(),
            temperature: 0.9,
            max_tokens: 8192,
            system_prompt: None,
            workspace_root: None,
        };
        store
            .update_conversation_settings(&conv.id, &new_settings)
            .expect("Failed to update");

        let reloaded = store.load_conversation(&conv.id).expect("Failed to load");
        assert_eq!(reloaded.settings.model, "custom-model");
        assert_eq!(reloaded.settings.temperature, 0.9);
    }
}

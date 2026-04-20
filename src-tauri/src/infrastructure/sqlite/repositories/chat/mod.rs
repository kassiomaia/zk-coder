//! `SQLite` storage for chat conversations and threads.
//!
//! This module provides persistence for conversations, threads, and messages
//! using `SQLite`.

pub(crate) mod conversations;
mod helpers;
pub(crate) mod messages;
pub(crate) mod threads;

use std::sync::{Arc, Mutex};

use rusqlite::Connection;
use thiserror::Error;

use crate::infrastructure::sqlite::migrations::{chat, chat_schema};

pub use conversations::{
    create_conversation, delete_conversation, load_all_conversations, load_conversation,
    update_conversation_archived, update_conversation_settings, update_conversation_title,
    update_conversation_workspace,
};

pub use threads::{
    create_thread, create_thread_with_message, delete_thread, load_thread, load_threads,
    rename_thread, set_thread_agent_session, update_thread_archived, update_thread_resolved,
};

pub use messages::{
    add_message, delete_message, load_messages, load_thread_feedback, load_thread_messages,
    update_message_feedback, update_message_status,
};

use crate::core::chat::entities::{
    ChatDomainError, ChatMessage, Conversation, ConversationSettings, MessageFeedback, MessageRole,
    MessageStatus, Thread,
};

use crate::core::chat::repositories::{
    ChatRepository, MessageRepository, RepositoryResult, ThreadRepository,
};

/// Errors from the `SQLite` chat repository.
#[derive(Debug, Error)]
pub enum ChatRepositoryError {
    /// `SQLite` error
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

impl From<ChatRepositoryError> for ChatDomainError {
    fn from(err: ChatRepositoryError) -> Self {
        match err {
            ChatRepositoryError::Sqlite(e) => Self::InvalidInput {
                reason: format!("Database error: {e}"),
            },
            ChatRepositoryError::NotFound(entity, id) => match entity {
                "Conversation" => Self::ConversationNotFound { id },
                "Thread" => Self::ThreadNotFound { id },
                "Message" => Self::MessageNotFound { id },
                _ => Self::InvalidInput {
                    reason: format!("{entity} not found: {id}"),
                },
            },
            ChatRepositoryError::LockError(e) => Self::InvalidInput {
                reason: format!("Lock error: {e}"),
            },
            ChatRepositoryError::Json(e) => Self::InvalidInput {
                reason: format!("JSON error: {e}"),
            },
            ChatRepositoryError::ThreadResolved(thread_title) => {
                Self::ThreadResolved { thread_title }
            }
        }
    }
}

#[allow(dead_code)]
/// SQLite-backed chat repository.
pub struct SqliteChatRepository {
    /// Database connection (protected by mutex for thread safety)
    conn: Arc<Mutex<Connection>>,
}

impl SqliteChatRepository {
    /// Creates a new chat repository at the given path.
    ///
    /// # Errors
    ///
    /// Returns an error if the database cannot be opened or initialized.
    pub fn new(db_path: &str) -> Result<Self, ChatRepositoryError> {
        let conn = Connection::open(db_path)?;
        conn.execute_batch("PRAGMA journal_mode=WAL;")?;
        chat_schema::init_schema(&conn)?;

        let store = Self {
            conn: Arc::new(Mutex::new(conn)),
        };

        // Run migration for orphan messages
        store.migrate_orphan_messages_to_threads()?;

        // Enable FK enforcement after migrations complete
        store
            .acquire_lock()?
            .execute_batch("PRAGMA foreign_keys=ON;")?;

        Ok(store)
    }

    /// Creates a chat repository from a shared connection.
    ///
    /// The schema and migrations must already have been applied via `migrations::init_all`.
    ///
    /// # Errors
    ///
    /// Returns an error if the database lock cannot be acquired.
    pub fn from_shared(conn: Arc<Mutex<Connection>>) -> Result<Self, ChatRepositoryError> {
        conn.lock()
            .map_err(|_| ChatRepositoryError::LockError("Failed to acquire lock".to_string()))?;
        Ok(Self { conn })
    }

    /// Creates an in-memory chat store (for testing).
    ///
    /// # Errors
    ///
    /// Returns an error if the database cannot be initialized.
    pub fn in_memory() -> Result<Self, ChatRepositoryError> {
        let conn = Connection::open_in_memory()?;
        chat_schema::init_schema(&conn)?;
        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    /// Acquires the database lock, returning a formatted error on failure.
    fn acquire_lock(&self) -> Result<std::sync::MutexGuard<'_, Connection>, ChatRepositoryError> {
        self.conn
            .lock()
            .map_err(|_| ChatRepositoryError::LockError("Failed to acquire lock".to_string()))
    }

    // ========================================================================
    // Conversation Operations
    // ========================================================================

    /// Creates a new conversation.
    pub fn create_conversation_impl(
        &self,
        title: &str,
    ) -> Result<Conversation, ChatRepositoryError> {
        let conn = self.acquire_lock()?;
        conversations::create_conversation(&conn, title)
    }

    /// Loads a conversation by ID.
    pub fn load_conversation_impl(&self, id: &str) -> Result<Conversation, ChatRepositoryError> {
        let conn = self.acquire_lock()?;
        conversations::load_conversation(&conn, id)
    }

    /// Loads all conversations, sorted by update time (newest first).
    pub fn load_all_conversations_impl(&self) -> Result<Vec<Conversation>, ChatRepositoryError> {
        let conn = self.acquire_lock()?;
        conversations::load_all_conversations(&conn)
    }

    /// Updates a conversation's title.
    pub fn update_conversation_title_impl(
        &self,
        id: &str,
        title: &str,
    ) -> Result<Conversation, ChatRepositoryError> {
        let conn = self.acquire_lock()?;
        conversations::update_conversation_title(&conn, id, title)
    }

    /// Deletes a conversation and all its threads and messages.
    pub fn delete_conversation_impl(&self, id: &str) -> Result<(), ChatRepositoryError> {
        let conn = self.acquire_lock()?;
        conversations::delete_conversation(&conn, id)
    }

    /// Updates a conversation's settings.
    pub fn update_conversation_settings_impl(
        &self,
        id: &str,
        settings: &ConversationSettings,
    ) -> Result<Conversation, ChatRepositoryError> {
        let conn = self.acquire_lock()?;
        conversations::update_conversation_settings(&conn, id, settings)
    }

    /// Updates a conversation's workspace root.
    pub fn update_conversation_workspace_impl(
        &self,
        id: &str,
        workspace_root: &str,
    ) -> Result<Conversation, ChatRepositoryError> {
        let conn = self.acquire_lock()?;
        conversations::update_conversation_workspace(&conn, id, workspace_root)
    }

    /// Updates a conversation's archived status.
    pub fn update_conversation_archived_impl(
        &self,
        id: &str,
        archived: bool,
    ) -> Result<(), ChatRepositoryError> {
        let conn = self.acquire_lock()?;
        conversations::update_conversation_archived(&conn, id, archived)
    }

    // ========================================================================
    // Thread Operations
    // ========================================================================

    /// Creates a new thread in a conversation.
    pub fn create_thread_impl(
        &self,
        conversation_id: &str,
        title: &str,
    ) -> Result<Thread, ChatRepositoryError> {
        let conn = self.acquire_lock()?;
        threads::create_thread(&conn, conversation_id, title)
    }

    /// Loads all threads for a conversation.
    pub fn load_threads_impl(
        &self,
        conversation_id: &str,
    ) -> Result<Vec<Thread>, ChatRepositoryError> {
        let conn = self.acquire_lock()?;
        threads::load_threads(&conn, conversation_id)
    }

    /// Loads a single thread by ID.
    pub fn load_thread_impl(&self, id: &str) -> Result<Thread, ChatRepositoryError> {
        let conn = self.acquire_lock()?;
        threads::load_thread(&conn, id)
    }

    /// Updates a thread's resolved status.
    pub fn update_thread_resolved_impl(
        &self,
        id: &str,
        resolved: bool,
    ) -> Result<(), ChatRepositoryError> {
        let conn = self.acquire_lock()?;
        threads::update_thread_resolved(&conn, id, resolved)
    }

    /// Updates a thread's archived status.
    pub fn update_thread_archived_impl(
        &self,
        id: &str,
        archived: bool,
    ) -> Result<(), ChatRepositoryError> {
        let conn = self.acquire_lock()?;
        threads::update_thread_archived(&conn, id, archived)
    }

    /// Sets the agent session ID for a thread.
    pub fn set_thread_agent_session_impl(
        &self,
        id: &str,
        agent_session_id: Option<String>,
    ) -> Result<(), ChatRepositoryError> {
        let conn = self.acquire_lock()?;
        threads::set_thread_agent_session(&conn, id, agent_session_id)
    }

    /// Updates a thread's YOLO mode status.
    pub fn update_thread_yolo_mode_impl(
        &self,
        id: &str,
        yolo_mode: bool,
    ) -> Result<(), ChatRepositoryError> {
        let conn = self.acquire_lock()?;
        threads::update_thread_yolo_mode(&conn, id, yolo_mode)
    }

    /// Deletes a thread and all its messages.
    pub fn delete_thread_impl(&self, id: &str) -> Result<(), ChatRepositoryError> {
        let conn = self.acquire_lock()?;
        threads::delete_thread(&conn, id)
    }

    /// Renames a thread.
    pub fn rename_thread_impl(&self, id: &str, title: &str) -> Result<Thread, ChatRepositoryError> {
        let conn = self.acquire_lock()?;
        threads::rename_thread(&conn, id, title)
    }

    /// Creates a thread with its first message atomically.
    pub fn create_thread_with_message_impl(
        &self,
        conversation_id: &str,
        thread_title: &str,
        message_content: &str,
        message_role: MessageRole,
    ) -> Result<(Thread, ChatMessage), ChatRepositoryError> {
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
    pub fn add_message_impl(&self, message: &ChatMessage) -> Result<(), ChatRepositoryError> {
        let conn = self.acquire_lock()?;
        messages::add_message(&conn, message)
    }

    /// Loads messages for a conversation.
    pub fn load_messages_impl(
        &self,
        conversation_id: &str,
    ) -> Result<Vec<ChatMessage>, ChatRepositoryError> {
        let conn = self.acquire_lock()?;
        messages::load_messages(&conn, conversation_id)
    }

    /// Loads messages for a specific thread.
    pub fn load_thread_messages_impl(
        &self,
        thread_id: &str,
    ) -> Result<Vec<ChatMessage>, ChatRepositoryError> {
        let conn = self.acquire_lock()?;
        messages::load_thread_messages(&conn, thread_id)
    }

    /// Deletes a message.
    pub fn delete_message_impl(&self, id: &str) -> Result<(), ChatRepositoryError> {
        let conn = self.acquire_lock()?;
        messages::delete_message(&conn, id)
    }

    /// Updates a message's status.
    pub fn update_message_status_impl(
        &self,
        id: &str,
        status: MessageStatus,
    ) -> Result<(), ChatRepositoryError> {
        let conn = self.acquire_lock()?;
        messages::update_message_status(&conn, id, status)
    }

    /// Updates a message's feedback (like/dislike).
    pub fn update_message_feedback_impl(
        &self,
        id: &str,
        feedback: Option<MessageFeedback>,
    ) -> Result<(), ChatRepositoryError> {
        let conn = self.acquire_lock()?;
        messages::update_message_feedback(&conn, id, feedback)
    }

    /// Loads messages with feedback for a specific thread.
    pub fn load_thread_feedback_impl(
        &self,
        thread_id: &str,
        limit: usize,
    ) -> Result<Vec<(ChatMessage, MessageFeedback)>, ChatRepositoryError> {
        let conn = self.acquire_lock()?;
        messages::load_thread_feedback(&conn, thread_id, limit)
    }

    // ========================================================================
    // Migration Operations
    // ========================================================================

    /// Migrates orphan messages to new threads.
    pub fn migrate_orphan_messages_to_threads(&self) -> Result<(), ChatRepositoryError> {
        let conn = self.acquire_lock()?;
        chat::migrate_orphan_messages_to_threads(&conn)
    }
}

// ========================================================================
// Trait Implementations
// ========================================================================

impl ChatRepository for SqliteChatRepository {
    fn create_conversation(&self, title: &str) -> RepositoryResult<Conversation> {
        self.create_conversation_impl(title)
            .map_err(ChatDomainError::from)
    }

    fn load_conversation(&self, id: &str) -> RepositoryResult<Conversation> {
        self.load_conversation_impl(id)
            .map_err(ChatDomainError::from)
    }

    fn load_all_conversations(&self) -> RepositoryResult<Vec<Conversation>> {
        self.load_all_conversations_impl()
            .map_err(ChatDomainError::from)
    }

    fn update_conversation_title(&self, id: &str, title: &str) -> RepositoryResult<Conversation> {
        self.update_conversation_title_impl(id, title)
            .map_err(ChatDomainError::from)
    }

    fn delete_conversation(&self, id: &str) -> RepositoryResult<()> {
        self.delete_conversation_impl(id)
            .map_err(ChatDomainError::from)
    }

    fn update_conversation_settings(
        &self,
        id: &str,
        settings: &ConversationSettings,
    ) -> RepositoryResult<Conversation> {
        self.update_conversation_settings_impl(id, settings)
            .map_err(ChatDomainError::from)
    }

    fn update_conversation_workspace(
        &self,
        id: &str,
        workspace_root: &str,
    ) -> RepositoryResult<Conversation> {
        self.update_conversation_workspace_impl(id, workspace_root)
            .map_err(ChatDomainError::from)
    }

    fn update_conversation_archived(&self, id: &str, archived: bool) -> RepositoryResult<()> {
        self.update_conversation_archived_impl(id, archived)
            .map_err(ChatDomainError::from)
    }
}

impl ThreadRepository for SqliteChatRepository {
    fn create_thread(&self, conversation_id: &str, title: &str) -> RepositoryResult<Thread> {
        self.create_thread_impl(conversation_id, title)
            .map_err(ChatDomainError::from)
    }

    fn load_threads(&self, conversation_id: &str) -> RepositoryResult<Vec<Thread>> {
        self.load_threads_impl(conversation_id)
            .map_err(ChatDomainError::from)
    }

    fn load_thread(&self, id: &str) -> RepositoryResult<Thread> {
        self.load_thread_impl(id).map_err(ChatDomainError::from)
    }

    fn update_thread_resolved(&self, id: &str, resolved: bool) -> RepositoryResult<()> {
        self.update_thread_resolved_impl(id, resolved)
            .map_err(ChatDomainError::from)
    }

    fn update_thread_archived(&self, id: &str, archived: bool) -> RepositoryResult<()> {
        self.update_thread_archived_impl(id, archived)
            .map_err(ChatDomainError::from)
    }

    fn set_thread_agent_session(
        &self,
        id: &str,
        agent_session_id: Option<String>,
    ) -> RepositoryResult<()> {
        self.set_thread_agent_session_impl(id, agent_session_id)
            .map_err(ChatDomainError::from)
    }

    fn update_thread_yolo_mode(&self, id: &str, yolo_mode: bool) -> RepositoryResult<()> {
        self.update_thread_yolo_mode_impl(id, yolo_mode)
            .map_err(ChatDomainError::from)
    }

    fn delete_thread(&self, id: &str) -> RepositoryResult<()> {
        self.delete_thread_impl(id).map_err(ChatDomainError::from)
    }

    fn rename_thread(&self, id: &str, title: &str) -> RepositoryResult<Thread> {
        self.rename_thread_impl(id, title)
            .map_err(ChatDomainError::from)
    }

    fn create_thread_with_message(
        &self,
        conversation_id: &str,
        thread_title: &str,
        message_content: &str,
        message_role: MessageRole,
    ) -> RepositoryResult<(Thread, ChatMessage)> {
        self.create_thread_with_message_impl(
            conversation_id,
            thread_title,
            message_content,
            message_role,
        )
        .map_err(ChatDomainError::from)
    }
}

impl MessageRepository for SqliteChatRepository {
    fn add_message(&self, message: &ChatMessage) -> RepositoryResult<()> {
        self.add_message_impl(message)
            .map_err(ChatDomainError::from)
    }

    fn load_messages(&self, conversation_id: &str) -> RepositoryResult<Vec<ChatMessage>> {
        self.load_messages_impl(conversation_id)
            .map_err(ChatDomainError::from)
    }

    fn load_thread_messages(&self, thread_id: &str) -> RepositoryResult<Vec<ChatMessage>> {
        self.load_thread_messages_impl(thread_id)
            .map_err(ChatDomainError::from)
    }

    fn delete_message(&self, id: &str) -> RepositoryResult<()> {
        self.delete_message_impl(id).map_err(ChatDomainError::from)
    }

    fn update_message_status(&self, id: &str, status: MessageStatus) -> RepositoryResult<()> {
        self.update_message_status_impl(id, status)
            .map_err(ChatDomainError::from)
    }

    fn update_message_feedback(
        &self,
        id: &str,
        feedback: Option<MessageFeedback>,
    ) -> RepositoryResult<()> {
        self.update_message_feedback_impl(id, feedback)
            .map_err(ChatDomainError::from)
    }
}

// The blanket impl in core/chat/repositories.rs will automatically implement

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_create_and_load_conversation() {
        let store = SqliteChatRepository::in_memory().expect("Failed to create store");
        let conv = store
            .create_conversation("Test Conversation")
            .expect("Failed to create");
        assert_eq!(conv.title, "Test Conversation");

        let loaded = store.load_conversation(&conv.id).expect("Failed to load");
        assert_eq!(loaded.id, conv.id);
        assert_eq!(loaded.title, conv.title);
    }

    #[test]
    fn test_create_and_load_thread() {
        let store = SqliteChatRepository::in_memory().expect("Failed to create store");
        let conv = store.create_conversation("Test").expect("Failed to create");
        let thread = store
            .create_thread(&conv.id, "Test Thread")
            .expect("Failed to create thread");
        assert_eq!(thread.conversation_id, conv.id);

        let threads = store.load_threads(&conv.id).expect("Failed to load");
        assert_eq!(threads.len(), 1);
    }

    #[test]
    fn test_add_and_load_messages() {
        let store = SqliteChatRepository::in_memory().expect("Failed to create store");
        let conv = store.create_conversation("Test").expect("Failed to create");
        let msg = ChatMessage::user(&conv.id, "Hello world");
        store.add_message(&msg).expect("Failed to add message");

        let messages = store.load_messages(&conv.id).expect("Failed to load");
        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0].content, "Hello world");
    }

    #[test]
    fn test_thread_resolved_error_message() {
        let error = ChatRepositoryError::ThreadResolved("thread-123".to_string());
        assert!(error.to_string().contains("resolved"));
        assert!(error.to_string().contains("thread-123"));
    }

    #[test]
    fn test_conversation_settings_persistence() {
        let store = SqliteChatRepository::in_memory().expect("Failed to create store");
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

    #[test]
    fn test_archived_thread_stays_hidden_after_repository_reopen() {
        let db_path = std::env::temp_dir().join(format!(
            "coder-chat-archive-regression-{}.db",
            uuid::Uuid::new_v4()
        ));

        {
            let store = SqliteChatRepository::new(db_path.to_str().expect("Invalid temp path"))
                .expect("Failed to create store");
            let conv = store
                .create_conversation("Archive Regression")
                .expect("Failed to create conversation");
            let thread = store
                .create_thread(&conv.id, "Should stay archived")
                .expect("Failed to create thread");

            store
                .add_message(&ChatMessage::user(&conv.id, "hello").with_thread(&thread.id))
                .expect("Failed to add message");
            store
                .update_thread_archived(&thread.id, true)
                .expect("Failed to archive thread");

            let threads = store
                .load_threads(&conv.id)
                .expect("Failed to load threads");
            assert!(threads.is_empty());
        }

        {
            let reopened = SqliteChatRepository::new(db_path.to_str().expect("Invalid temp path"))
                .expect("Failed to reopen store");
            let conversations = reopened
                .load_all_conversations()
                .expect("Failed to load conversations");
            let conv_id = conversations
                .first()
                .expect("Expected persisted conversation")
                .id
                .clone();

            let threads = reopened
                .load_threads(&conv_id)
                .expect("Failed to load threads");
            assert!(
                threads.is_empty(),
                "Archived thread should not reappear after reopening the repository"
            );
        }

        let _ = fs::remove_file(db_path);
    }
}

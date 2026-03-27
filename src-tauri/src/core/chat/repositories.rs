//! Chat repository interfaces.
//!
//! These traits define the storage operations for chat entities.
//! Implementations are provided by the infrastructure layer.

use super::entities::{
    ChatDomainError, ChatMessage, Conversation, ConversationSettings, MessageFeedback, MessageRole,
    MessageStatus, Thread,
};

/// Result type for repository operations.
pub type RepositoryResult<T> = Result<T, ChatDomainError>;

/// Repository for conversation operations.
pub trait ChatRepository: Send + Sync {
    /// Creates a new conversation.
    fn create_conversation(&self, title: &str) -> RepositoryResult<Conversation>;

    /// Loads a conversation by ID.
    fn load_conversation(&self, id: &str) -> RepositoryResult<Conversation>;

    /// Loads all conversations, sorted by update time (newest first).
    fn load_all_conversations(&self) -> RepositoryResult<Vec<Conversation>>;

    /// Updates a conversation's title.
    fn update_conversation_title(&self, id: &str, title: &str) -> RepositoryResult<Conversation>;

    /// Deletes a conversation and all its threads and messages.
    fn delete_conversation(&self, id: &str) -> RepositoryResult<()>;

    /// Updates a conversation's settings.
    fn update_conversation_settings(
        &self,
        id: &str,
        settings: &ConversationSettings,
    ) -> RepositoryResult<Conversation>;

    /// Updates a conversation's workspace root.
    fn update_conversation_workspace(
        &self,
        id: &str,
        workspace_root: &str,
    ) -> RepositoryResult<Conversation>;

    /// Updates a conversation's archived status.
    fn update_conversation_archived(&self, id: &str, archived: bool) -> RepositoryResult<()>;
}

/// Repository for thread operations.
pub trait ThreadRepository: Send + Sync {
    /// Creates a new thread in a conversation.
    fn create_thread(&self, conversation_id: &str, title: &str) -> RepositoryResult<Thread>;

    /// Loads all threads for a conversation.
    fn load_threads(&self, conversation_id: &str) -> RepositoryResult<Vec<Thread>>;

    /// Loads a single thread by ID.
    fn load_thread(&self, id: &str) -> RepositoryResult<Thread>;

    /// Updates a thread's resolved status.
    fn update_thread_resolved(&self, id: &str, resolved: bool) -> RepositoryResult<()>;

    /// Updates a thread's archived status.
    fn update_thread_archived(&self, id: &str, archived: bool) -> RepositoryResult<()>;

    /// Sets the agent session ID for a thread.
    fn set_thread_agent_session(
        &self,
        id: &str,
        agent_session_id: Option<String>,
    ) -> RepositoryResult<()>;

    /// Deletes a thread and all its messages.
    fn delete_thread(&self, id: &str) -> RepositoryResult<()>;

    /// Renames a thread.
    fn rename_thread(&self, id: &str, title: &str) -> RepositoryResult<Thread>;

    /// Creates a thread with its first message atomically.
    fn create_thread_with_message(
        &self,
        conversation_id: &str,
        thread_title: &str,
        message_content: &str,
        message_role: MessageRole,
    ) -> RepositoryResult<(Thread, ChatMessage)>;
}

/// Repository for message operations.
pub trait MessageRepository: Send + Sync {
    /// Adds a message to a conversation (optionally to a thread).
    fn add_message(&self, message: &ChatMessage) -> RepositoryResult<()>;

    /// Loads messages for a conversation.
    fn load_messages(&self, conversation_id: &str) -> RepositoryResult<Vec<ChatMessage>>;

    /// Loads messages for a specific thread.
    fn load_thread_messages(&self, thread_id: &str) -> RepositoryResult<Vec<ChatMessage>>;

    /// Deletes a message.
    fn delete_message(&self, id: &str) -> RepositoryResult<()>;

    /// Updates a message's status.
    fn update_message_status(&self, id: &str, status: MessageStatus) -> RepositoryResult<()>;

    /// Updates a message's feedback (like/dislike).
    fn update_message_feedback(
        &self,
        id: &str,
        feedback: Option<MessageFeedback>,
    ) -> RepositoryResult<()>;
}

/// Combined chat repository contract.
///
/// This is a convenience trait for adapters that implement all chat persistence
/// operations behind a single dependency.
pub trait ChatRepositorySet: ChatRepository + ThreadRepository + MessageRepository {}

// Blanket implementation for types that implement all traits
impl<T> ChatRepositorySet for T where T: ChatRepository + ThreadRepository + MessageRepository {}

//! Chat service interface.
//!
//! Defines the business operations for chat functionality.

use super::entities::{
    ChatMessage, Conversation, ConversationSettings, MessageFeedback, MessageRole, MessageStatus,
    Thread,
};
use super::repositories::RepositoryResult;

/// Request to create a new thread.
#[derive(Debug, Clone)]
pub struct CreateThreadRequest {
    /// ID of the parent conversation
    pub conversation_id: String,
    /// Title for the thread
    pub title: String,
}

/// Request to send a message.
#[derive(Debug, Clone)]
pub struct SendMessageRequest {
    /// ID of the conversation
    pub conversation_id: String,
    /// ID of the thread
    pub thread_id: String,
    /// Message content
    pub content: String,
    /// Message role (user, assistant, system)
    pub role: MessageRole,
}

/// Chat service interface.
///
/// This trait defines the business logic operations for chat functionality.
/// Implementations should coordinate between repositories and enforce business rules.
pub trait ChatService: Send + Sync {
    // ========================================================================
    // Conversation Operations
    // ========================================================================

    /// Creates a new conversation.
    fn create_conversation(&self, title: &str) -> RepositoryResult<Conversation>;

    /// Loads a conversation by ID.
    fn load_conversation(&self, id: &str) -> RepositoryResult<Conversation>;

    /// Loads all conversations.
    fn load_all_conversations(&self) -> RepositoryResult<Vec<Conversation>>;

    /// Updates a conversation's title.
    fn rename_conversation(&self, id: &str, title: &str) -> RepositoryResult<Conversation>;

    /// Deletes a conversation.
    fn delete_conversation(&self, id: &str) -> RepositoryResult<()>;

    /// Updates a conversation's settings.
    fn update_conversation_settings(
        &self,
        id: &str,
        settings: &ConversationSettings,
    ) -> RepositoryResult<Conversation>;

    /// Updates a conversation's workspace root.
    fn set_conversation_workspace(
        &self,
        id: &str,
        workspace_root: &str,
    ) -> RepositoryResult<Conversation>;

    // ========================================================================
    // Thread Operations
    // ========================================================================

    /// Creates a new thread.
    fn create_thread(&self, request: CreateThreadRequest) -> RepositoryResult<Thread>;

    /// Loads threads for a conversation.
    fn load_threads(&self, conversation_id: &str) -> RepositoryResult<Vec<Thread>>;

    /// Loads a single thread by ID.
    fn load_thread(&self, id: &str) -> RepositoryResult<Thread>;

    /// Updates a thread's resolved status.
    fn resolve_thread(&self, id: &str, resolved: bool) -> RepositoryResult<()>;

    /// Deletes a thread.
    fn delete_thread(&self, id: &str) -> RepositoryResult<()>;

    /// Renames a thread.
    fn rename_thread(&self, id: &str, title: &str) -> RepositoryResult<Thread>;

    /// Creates a thread with its first message atomically.
    fn create_thread_with_message(
        &self,
        conversation_id: &str,
        thread_title: &str,
        content: &str,
        role: MessageRole,
    ) -> RepositoryResult<(Thread, ChatMessage)>;

    /// Sets or clears the agent session for a thread.
    fn set_thread_agent_session(
        &self,
        id: &str,
        agent_session_id: Option<String>,
    ) -> RepositoryResult<()>;

    // ========================================================================
    // Message Operations
    // ========================================================================

    /// Sends a message (with thread validation).
    ///
    /// # Errors
    ///
    /// Returns `ChatDomainError::ThreadResolved` if the thread is resolved.
    fn send_message(&self, request: SendMessageRequest) -> RepositoryResult<ChatMessage>;

    /// Loads messages for a conversation.
    fn load_messages(&self, conversation_id: &str) -> RepositoryResult<Vec<ChatMessage>>;

    /// Loads messages for a thread.
    fn load_thread_messages(&self, thread_id: &str) -> RepositoryResult<Vec<ChatMessage>>;

    /// Persists a message through the chat service boundary.
    fn add_message(&self, message: &ChatMessage) -> RepositoryResult<()>;

    /// Deletes a message.
    fn delete_message(&self, id: &str) -> RepositoryResult<()>;

    /// Updates a message's status.
    fn update_message_status(&self, id: &str, status: MessageStatus) -> RepositoryResult<()>;

    /// Updates a message's feedback.
    fn update_message_feedback(
        &self,
        id: &str,
        feedback: Option<MessageFeedback>,
    ) -> RepositoryResult<()>;
}

//! Defines the service interface for the chat module.
//!
//! This module provides the `ChatService` trait, which outlines the core
//! business logic operations for managing chat functionality. Implementations
//! of this trait are responsible for coordinating between repositories and
// anana
//! enforcing business rules.

use super::entities::{
    ChatMessage, Conversation, ConversationSettings, MessageFeedback, MessageRole, MessageStatus,
    Thread,
};
use super::repositories::RepositoryResult;

/// A request to create a new thread.
#[derive(Debug, Clone)]
pub struct CreateThreadRequest {
    /// The ID of the parent conversation.
    pub conversation_id: String,
    /// The title for the new thread.
    pub title: String,
}

/// A request to send a message.
#[derive(Debug, Clone)]
pub struct SendMessageRequest {
    /// The ID of the conversation to send the message to.
    pub conversation_id: String,
    /// The ID of the thread to send the message to.
    pub thread_id: String,
    /// The content of the message.
    pub content: String,
    /// The role of the message sender (e.g., user, assistant).
    pub role: MessageRole,
}

/// The main service interface for chat-related business logic.
///
/// This trait defines the high-level operations for managing conversations,
/// threads, and messages, abstracting away the underlying implementation details.
pub trait ChatService: Send + Sync {
    // ========================================================================
    // Conversation Operations
    // ========================================================================

    /// Creates a new conversation with the given title.
    fn create_conversation(&self, title: &str) -> RepositoryResult<Conversation>;

    /// Loads a conversation by its unique ID.
    fn load_conversation(&self, id: &str) -> RepositoryResult<Conversation>;

    /// Loads all conversations.
    fn load_all_conversations(&self) -> RepositoryResult<Vec<Conversation>>;

    /// Renames a conversation.
    fn rename_conversation(&self, id: &str, title: &str) -> RepositoryResult<Conversation>;

    /// Deletes a conversation and all its associated data.
    fn delete_conversation(&self, id: &str) -> RepositoryResult<()>;

    /// Updates the settings for a conversation.
    fn update_conversation_settings(
        &self,
        id: &str,
        settings: &ConversationSettings,
    ) -> RepositoryResult<Conversation>;

    /// Sets the workspace root directory for a conversation.
    fn set_conversation_workspace(
        &self,
        id: &str,
        workspace_root: &str,
    ) -> RepositoryResult<Conversation>;

    /// Archives or restores a conversation.
    fn archive_conversation(&self, id: &str, archived: bool) -> RepositoryResult<()>;

    // ========================================================================
    // Thread Operations
    // ========================================================================

    /// Creates a new thread based on the provided request.
    fn create_thread(&self, request: CreateThreadRequest) -> RepositoryResult<Thread>;

    /// Loads all threads for a specific conversation.
    fn load_threads(&self, conversation_id: &str) -> RepositoryResult<Vec<Thread>>;

    /// Loads a single thread by its unique ID.
    fn load_thread(&self, id: &str) -> RepositoryResult<Thread>;

    /// Updates the resolved status of a thread.
    fn resolve_thread(&self, id: &str, resolved: bool) -> RepositoryResult<()>;

    /// Archives or restores a thread.
    fn archive_thread(&self, id: &str, archived: bool) -> RepositoryResult<()>;

    /// Deletes a thread and its associated messages.
    fn delete_thread(&self, id: &str) -> RepositoryResult<()>;

    /// Renames a thread.
    fn rename_thread(&self, id: &str, title: &str) -> RepositoryResult<Thread>;

    /// Creates a new thread and its first message in a single, atomic operation.
    fn create_thread_with_message(
        &self,
        conversation_id: &str,
        thread_title: &str,
        content: &str,
        role: MessageRole,
    ) -> RepositoryResult<(Thread, ChatMessage)>;

    /// Sets or clears the agent session ID for a thread.
    fn set_thread_agent_session(
        &self,
        id: &str,
        agent_session_id: Option<String>,
    ) -> RepositoryResult<()>;

    /// Sets the YOLO mode status for a thread.
    fn set_yolo_mode(&self, id: &str, yolo_mode: bool) -> RepositoryResult<()>;

    // ========================================================================
    // Message Operations
    // ========================================================================

    /// Sends a message, validating that the target thread is not resolved.
    ///
    /// # Errors
    ///
    /// Returns `ChatDomainError::ThreadResolved` if the thread is already resolved.
    fn send_message(&self, request: SendMessageRequest) -> RepositoryResult<ChatMessage>;

    /// Loads all messages for a specific conversation.
    fn load_messages(&self, conversation_id: &str) -> RepositoryResult<Vec<ChatMessage>>;

    /// Loads all messages for a specific thread.
    fn load_thread_messages(&self, thread_id: &str) -> RepositoryResult<Vec<ChatMessage>>;

    /// Persists a message through the chat service boundary.
    fn add_message(&self, message: &ChatMessage) -> RepositoryResult<()>;

    /// Deletes a message by its unique ID.
    fn delete_message(&self, id: &str) -> RepositoryResult<()>;

    /// Updates the status of a message.
    fn update_message_status(&self, id: &str, status: MessageStatus) -> RepositoryResult<()>;

    /// Updates the feedback for a message.
    fn update_message_feedback(
        &self,
        id: &str,
        feedback: Option<MessageFeedback>,
    ) -> RepositoryResult<()>;
}

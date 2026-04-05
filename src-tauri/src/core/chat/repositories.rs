//! Defines the repository interfaces for the chat module.
//!
//! These traits outline the storage and retrieval operations for chat entities,
//! such as conversations, threads, and messages. The actual implementations
//! are provided by the infrastructure layer, decoupling the core domain logic
//! from the specific persistence mechanism.

use super::entities::{
    ChatDomainError, ChatMessage, Conversation, ConversationSettings, MessageFeedback, MessageRole,
    MessageStatus, Thread,
};

/// A specialized `Result` type for chat repository operations.
pub type RepositoryResult<T> = Result<T, ChatDomainError>;

/// A repository for managing `Conversation` data.
pub trait ChatRepository: Send + Sync {
    /// Creates a new conversation with the given title.
    fn create_conversation(&self, title: &str) -> RepositoryResult<Conversation>;

    /// Loads a conversation by its unique ID.
    fn load_conversation(&self, id: &str) -> RepositoryResult<Conversation>;

    /// Loads all conversations, sorted by their last update time (newest first).
    fn load_all_conversations(&self) -> RepositoryResult<Vec<Conversation>>;

    /// Updates the title of a conversation.
    fn update_conversation_title(&self, id: &str, title: &str) -> RepositoryResult<Conversation>;

    /// Deletes a conversation and all of its associated threads and messages.
    fn delete_conversation(&self, id: &str) -> RepositoryResult<()>;

    /// Updates the settings for a conversation.
    fn update_conversation_settings(
        &self,
        id: &str,
        settings: &ConversationSettings,
    ) -> RepositoryResult<Conversation>;

    /// Updates the workspace root directory for a conversation.
    fn update_conversation_workspace(
        &self,
        id: &str,
        workspace_root: &str,
    ) -> RepositoryResult<Conversation>;

    /// Updates the archived status of a conversation.
    fn update_conversation_archived(&self, id: &str, archived: bool) -> RepositoryResult<()>;
}

/// A repository for managing `Thread` data.
pub trait ThreadRepository: Send + Sync {
    /// Creates a new thread within a conversation.
    fn create_thread(&self, conversation_id: &str, title: &str) -> RepositoryResult<Thread>;

    /// Loads all threads for a specific conversation.
    fn load_threads(&self, conversation_id: &str) -> RepositoryResult<Vec<Thread>>;

    /// Loads a single thread by its unique ID.
    fn load_thread(&self, id: &str) -> RepositoryResult<Thread>;

    /// Updates the resolved status of a thread.
    fn update_thread_resolved(&self, id: &str, resolved: bool) -> RepositoryResult<()>;

    /// Updates the archived status of a thread.
    fn update_thread_archived(&self, id: &str, archived: bool) -> RepositoryResult<()>;

    /// Sets the agent session ID for a thread.
    fn set_thread_agent_session(
        &self,
        id: &str,
        agent_session_id: Option<String>,
    ) -> RepositoryResult<()>;

    /// Updates the YOLO mode status of a thread.
    fn update_thread_yolo_mode(&self, id: &str, yolo_mode: bool) -> RepositoryResult<()>;

    /// Deletes a thread and all of its associated messages.
    fn delete_thread(&self, id: &str) -> RepositoryResult<()>;

    /// Renames a thread.
    fn rename_thread(&self, id: &str, title: &str) -> RepositoryResult<Thread>;

    /// Creates a new thread and its first message in a single, atomic operation.
    fn create_thread_with_message(
        &self,
        conversation_id: &str,
        thread_title: &str,
        message_content: &str,
        message_role: MessageRole,
    ) -> RepositoryResult<(Thread, ChatMessage)>;
}

/// A repository for managing `ChatMessage` data.
pub trait MessageRepository: Send + Sync {
    /// Adds a new message to a conversation, optionally within a thread.
    fn add_message(&self, message: &ChatMessage) -> RepositoryResult<()>;

    /// Loads all messages for a specific conversation.
    fn load_messages(&self, conversation_id: &str) -> RepositoryResult<Vec<ChatMessage>>;

    /// Loads all messages for a specific thread.
    fn load_thread_messages(&self, thread_id: &str) -> RepositoryResult<Vec<ChatMessage>>;

    /// Deletes a message by its unique ID.
    fn delete_message(&self, id: &str) -> RepositoryResult<()>;

    /// Updates the status of a message.
    fn update_message_status(&self, id: &str, status: MessageStatus) -> RepositoryResult<()>;

    /// Updates the feedback (like/dislike) for a message.
    fn update_message_feedback(
        &self,
        id: &str,
        feedback: Option<MessageFeedback>,
    ) -> RepositoryResult<()>;
}

/// A combined repository contract for all chat-related persistence.
///
/// This trait serves as a convenient way to require all chat repository
/// capabilities behind a single dependency, which is useful for adapters
/// that implement all the necessary traits.
pub trait ChatRepositorySet: ChatRepository + ThreadRepository + MessageRepository {}

// A blanket implementation that applies `ChatRepositorySet` to any type
// that implements all the required underlying repository traits.
impl<T> ChatRepositorySet for T where T: ChatRepository + ThreadRepository + MessageRepository {}

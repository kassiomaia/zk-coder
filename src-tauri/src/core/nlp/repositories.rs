//! NLP repository interfaces.

use thiserror::Error;

use super::entities::ContextWindow;

/// Result type for repository operations.
pub type RepositoryResult<T> = Result<T, NlpDomainError>;

/// Errors that can occur in NLP domain operations.
#[derive(Debug, Clone, Error)]
pub enum NlpDomainError {
    /// Context window was not found for the given conversation.
    #[error("Context window not found for conversation: {conversation_id}")]
    ContextNotFound {
        /// The conversation ID that was not found.
        conversation_id: String,
    },
    /// Invalid input provided to NLP operations.
    #[error("Invalid NLP state: {reason}")]
    InvalidInput {
        /// The reason for the invalid input.
        reason: String,
    },
}

/// Repository for managing context windows for conversations.
pub trait ContextWindowRepository: Send + Sync {
    /// Load an existing context window for a conversation.
    fn load(&self, conversation_id: &str) -> RepositoryResult<ContextWindow>;
    /// Save a context window.
    fn save(&self, window: &ContextWindow) -> RepositoryResult<()>;
    /// Create a new context window for a conversation.
    fn create(&self, conversation_id: &str) -> RepositoryResult<ContextWindow>;
    /// Check if a context window exists for a conversation.
    fn exists(&self, conversation_id: &str) -> RepositoryResult<bool>;
    /// Delete a context window for a conversation.
    fn delete(&self, conversation_id: &str) -> RepositoryResult<()>;
}

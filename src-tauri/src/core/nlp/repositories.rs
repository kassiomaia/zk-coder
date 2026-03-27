//! NLP repository interfaces.

use thiserror::Error;

use super::entities::ContextWindow;

pub type RepositoryResult<T> = Result<T, NlpDomainError>;

#[derive(Debug, Clone, Error)]
pub enum NlpDomainError {
    #[error("Context window not found for conversation: {conversation_id}")]
    ContextNotFound { conversation_id: String },
    #[error("Invalid NLP state: {reason}")]
    InvalidInput { reason: String },
}

pub trait ContextWindowRepository: Send + Sync {
    fn load(&self, conversation_id: &str) -> RepositoryResult<ContextWindow>;
    fn save(&self, window: &ContextWindow) -> RepositoryResult<()>;
    fn create(&self, conversation_id: &str) -> RepositoryResult<ContextWindow>;
    fn exists(&self, conversation_id: &str) -> RepositoryResult<bool>;
    fn delete(&self, conversation_id: &str) -> RepositoryResult<()>;
}

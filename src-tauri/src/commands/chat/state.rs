use std::sync::Arc;

use crate::application::ChatServiceImpl;
use crate::core::chat::services::ChatService;
use crate::infrastructure::sqlite::repositories::chat::SqliteChatRepository;

/// Application state for chat commands.
pub struct ChatState {
    /// Chat service for business logic
    pub service: Arc<dyn ChatService>,
}

impl ChatState {
    /// Creates a new chat state with a file-backed database.
    ///
    /// # Errors
    ///
    /// Returns an error if the database cannot be initialized.
    pub fn new(db_path: &str) -> Result<Self, String> {
        let repository = Arc::new(SqliteChatRepository::new(db_path).map_err(|e| e.to_string())?);
        let service = Arc::new(ChatServiceImpl::new(repository)) as Arc<dyn ChatService>;
        Ok(Self { service })
    }

    /// Creates a new chat state with an in-memory database (for testing).
    ///
    /// # Errors
    ///
    /// Returns an error if the database cannot be initialized.
    pub fn in_memory() -> Result<Self, String> {
        let repository = Arc::new(SqliteChatRepository::in_memory().map_err(|e| e.to_string())?);
        let service = Arc::new(ChatServiceImpl::new(repository)) as Arc<dyn ChatService>;
        Ok(Self { service })
    }

    /// Creates a new chat state with a custom service (for testing).
    #[cfg(test)]
    pub fn with_service(service: Arc<dyn ChatService>) -> Self {
        Self { service }
    }
}

use std::sync::Arc;

use crate::core::config::ConfigRepository;
use crate::infrastructure::sqlite::repositories::config::SqliteConfigRepository;

/// Application state for configuration commands.
pub struct ConfigState {
    /// Configuration repository for persistence
    pub repository: Arc<dyn ConfigRepository>,
}

impl ConfigState {
    /// Creates a new config state.
    ///
    /// # Errors
    ///
    /// Returns an error if the store cannot be initialized.
    pub fn new(db_path: &str) -> Result<Self, String> {
        let repository = Arc::new(SqliteConfigRepository::new(db_path).map_err(|e| e.to_string())?)
            as Arc<dyn ConfigRepository>;
        Ok(Self { repository })
    }

    /// Creates a new config state with an in-memory database (for testing).
    ///
    /// # Errors
    ///
    /// Returns an error if the store cannot be initialized.
    pub fn in_memory() -> Result<Self, String> {
        let repository = Arc::new(SqliteConfigRepository::in_memory().map_err(|e| e.to_string())?)
            as Arc<dyn ConfigRepository>;
        Ok(Self { repository })
    }
}

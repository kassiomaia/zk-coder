//! SQLite storage for LLM provider configuration.
//!
//! This module provides persistence for providers, models, and operation defaults
//! using SQLite.

use std::sync::{Mutex, MutexGuard};

use rusqlite::Connection;
use thiserror::Error;

use crate::nlp::config::{
    ApiType, ManualModelEntry, Model, ModelCapabilities, ModelsSource, OperationDefault,
    OperationType, Provider,
};

// Re-export submodule functions
pub mod schema;
pub mod migrations;
pub mod providers;
pub mod models;
pub mod operation_defaults;
pub mod api_keys;
pub mod manual_models;
pub mod helpers;

// Re-export commonly used types
pub use schema::init_schema;
pub use migrations::{migrate_api_type_constraint, migrate_models_source, normalize_base_urls};
pub use helpers::parse_datetime;

/// Errors from the config store.
#[derive(Debug, Error)]
pub enum ConfigStoreError {
    /// SQLite error
    #[error("SQLite error: {0}")]
    Sqlite(#[from] rusqlite::Error),
    /// Not found error
    #[error("{0} not found: {1}")]
    NotFound(&'static str, String),
    /// Lock error
    #[error("Lock error: {0}")]
    LockError(String),
}

/// SQLite-based configuration store.
pub struct ConfigStore {
    /// Database connection (protected by mutex for thread safety)
    conn: Mutex<Connection>,
}

impl ConfigStore {
    /// Creates a new config store at the given path.
    ///
    /// # Errors
    ///
    /// Returns an error if the database cannot be opened or initialized.
    pub fn new(db_path: &str) -> Result<Self, ConfigStoreError> {
        let conn = Connection::open(db_path)?;
        init_schema(&conn)?;
        migrate_api_type_constraint(&conn)?;
        migrate_models_source(&conn)?;
        normalize_base_urls(&conn)?;
        let store = Self {
            conn: Mutex::new(conn),
        };
        store.seed_builtin_providers()?;
        Ok(store)
    }

    /// Creates an in-memory config store (for testing).
    ///
    /// # Errors
    ///
    /// Returns an error if the database cannot be initialized.
    pub fn in_memory() -> Result<Self, ConfigStoreError> {
        let conn = Connection::open_in_memory()?;
        init_schema(&conn)?;
        migrate_api_type_constraint(&conn)?;
        migrate_models_source(&conn)?;
        normalize_base_urls(&conn)?;
        let store = Self {
            conn: Mutex::new(conn),
        };
        store.seed_builtin_providers()?;
        Ok(store)
    }

    fn acquire_lock(&self) -> Result<MutexGuard<'_, Connection>, ConfigStoreError> {
        self.conn
            .lock()
            .map_err(|_| ConfigStoreError::LockError("Failed to acquire lock".to_string()))
    }

    fn seed_builtin_providers(&self) -> Result<(), ConfigStoreError> {
        let conn = self.acquire_lock()?;
        providers::seed_builtin_providers(&conn)?;
        Ok(())
    }

    // =========================================================================
    // Provider Operations
    // =========================================================================

    /// Gets all providers.
    ///
    /// # Errors
    ///
    /// Returns an error if the database operation fails.
    pub fn get_providers(&self) -> Result<Vec<Provider>, ConfigStoreError> {
        let conn = self.acquire_lock()?;
        providers::get_providers(&conn)
    }

    /// Gets a provider by ID.
    ///
    /// # Errors
    ///
    /// Returns an error if the database operation fails.
    pub fn get_provider(&self, id: &str) -> Result<Option<Provider>, ConfigStoreError> {
        let conn = self.acquire_lock()?;
        providers::get_provider(&conn, id)
    }

    /// Saves a provider (create or update).
    ///
    /// # Errors
    ///
    /// Returns an error if the database operation fails.
    pub fn save_provider(&self, provider: &Provider) -> Result<(), ConfigStoreError> {
        let conn = self.acquire_lock()?;
        providers::save_provider(&conn, provider)
    }

    /// Deletes a provider.
    ///
    /// # Errors
    ///
    /// Returns an error if the database operation fails.
    pub fn delete_provider(&self, id: &str) -> Result<(), ConfigStoreError> {
        let conn = self.acquire_lock()?;
        providers::delete_provider(&conn, id)
    }

    /// Sets the configured status of a provider.
    ///
    /// # Errors
    ///
    /// Returns an error if the database operation fails.
    pub fn set_provider_configured(
        &self,
        id: &str,
        configured: bool,
    ) -> Result<(), ConfigStoreError> {
        let conn = self.acquire_lock()?;
        providers::set_provider_configured(&conn, id, configured)
    }

    // =========================================================================
    // Model Operations
    // =========================================================================

    /// Gets all models for a provider.
    ///
    /// # Errors
    ///
    /// Returns an error if the database operation fails.
    pub fn get_models(&self, provider_id: &str) -> Result<Vec<Model>, ConfigStoreError> {
        let conn = self.acquire_lock()?;
        models::get_models(&conn, provider_id)
    }

    /// Saves models for a provider (replaces existing).
    ///
    /// # Errors
    ///
    /// Returns an error if the database operation fails.
    pub fn save_models(&self, provider_id: &str, models: &[Model]) -> Result<(), ConfigStoreError> {
        use rusqlite::params;
        let mut conn = self.acquire_lock()?;
        let tx = conn.transaction()?;
        tx.execute(
            "DELETE FROM models WHERE provider_id = ?1",
            params![provider_id],
        )?;

        for model in models {
            tx.execute(
                "INSERT INTO models (id, provider_id, name, supports_analysis, supports_code,
                 supports_vision, supports_audio, supports_tips, supports_review, last_fetched)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                params![
                    &model.id,
                    &model.provider_id,
                    &model.name,
                    model.capabilities.analysis,
                    model.capabilities.code,
                    model.capabilities.vision,
                    model.capabilities.audio,
                    model.capabilities.tips,
                    model.capabilities.review,
                    &model.last_fetched.to_rfc3339(),
                ],
            )?;
        }

        tx.commit()?;
        Ok(())
    }

    /// Adds a single model to the models table (without replacing existing models).
    ///
    /// # Errors
    ///
    /// Returns an error if the database operation fails.
    pub fn add_model(&self, model: &Model) -> Result<(), ConfigStoreError> {
        let conn = self.acquire_lock()?;
        models::add_model(&conn, model)
    }

    /// Removes a model from the models table by provider ID and model name.
    ///
    /// # Errors
    ///
    /// Returns an error if the database operation fails.
    pub fn delete_model_by_name(
        &self,
        provider_id: &str,
        model_name: &str,
    ) -> Result<(), ConfigStoreError> {
        let conn = self.acquire_lock()?;
        models::delete_model_by_name(&conn, provider_id, model_name)
    }

    /// Gets a model by ID.
    ///
    /// # Errors
    ///
    /// Returns an error if the database operation fails.
    pub fn get_model(&self, id: &str) -> Result<Option<Model>, ConfigStoreError> {
        let conn = self.acquire_lock()?;
        models::get_model(&conn, id)
    }

    /// Gets a model by provider ID and model name.
    ///
    /// # Errors
    ///
    /// Returns an error if the database operation fails.
    pub fn get_model_by_name(
        &self,
        provider_id: &str,
        name: &str,
    ) -> Result<Option<Model>, ConfigStoreError> {
        let conn = self.acquire_lock()?;
        models::get_model_by_name(&conn, provider_id, name)
    }

    // =========================================================================
    // Operation Default Operations
    // =========================================================================

    /// Gets all operation defaults.
    ///
    /// # Errors
    ///
    /// Returns an error if the database operation fails.
    pub fn get_operation_defaults(&self) -> Result<Vec<OperationDefault>, ConfigStoreError> {
        let conn = self.acquire_lock()?;
        operation_defaults::get_operation_defaults(&conn)
    }

    /// Sets the default model for an operation.
    ///
    /// # Errors
    ///
    /// Returns an error if the database operation fails.
    pub fn set_operation_default(
        &self,
        operation: OperationType,
        model_id: &str,
    ) -> Result<(), ConfigStoreError> {
        let conn = self.acquire_lock()?;
        operation_defaults::set_operation_default(&conn, operation, model_id)
    }

    /// Clears the default model for an operation.
    ///
    /// # Errors
    ///
    /// Returns an error if the database operation fails.
    pub fn clear_operation_default(
        &self,
        operation: OperationType,
    ) -> Result<(), ConfigStoreError> {
        let conn = self.acquire_lock()?;
        operation_defaults::clear_operation_default(&conn, operation)
    }

    /// Clears all operation defaults for models belonging to a provider.
    ///
    /// # Errors
    ///
    /// Returns an error if the database operation fails.
    pub fn clear_defaults_for_provider(&self, provider_id: &str) -> Result<(), ConfigStoreError> {
        let conn = self.acquire_lock()?;
        operation_defaults::clear_defaults_for_provider(&conn, provider_id)
    }

    // =========================================================================
    // API Key Operations
    // =========================================================================

    /// Stores an API key for a provider.
    ///
    /// # Errors
    ///
    /// Returns an error if the database operation fails.
    pub fn store_api_key(&self, provider_id: &str, api_key: &str) -> Result<(), ConfigStoreError> {
        let conn = self.acquire_lock()?;
        api_keys::store_api_key(&conn, provider_id, api_key)
    }

    /// Retrieves an API key for a provider.
    ///
    /// # Errors
    ///
    /// Returns an error if the key is not found or database operation fails.
    pub fn retrieve_api_key(&self, provider_id: &str) -> Result<String, ConfigStoreError> {
        let conn = self.acquire_lock()?;
        api_keys::retrieve_api_key(&conn, provider_id)
    }

    /// Deletes an API key for a provider.
    ///
    /// # Errors
    ///
    /// Returns an error if the database operation fails.
    pub fn delete_api_key(&self, provider_id: &str) -> Result<(), ConfigStoreError> {
        let conn = self.acquire_lock()?;
        api_keys::delete_api_key(&conn, provider_id)
    }

    /// Checks if an API key exists for a provider.
    ///
    /// # Errors
    ///
    /// Returns an error if the database operation fails.
    pub fn has_api_key(&self, provider_id: &str) -> Result<bool, ConfigStoreError> {
        let conn = self.acquire_lock()?;
        api_keys::has_api_key(&conn, provider_id)
    }

    // =========================================================================
    // Manual Model Operations
    // =========================================================================

    /// Gets all manual models for a provider.
    ///
    /// # Errors
    ///
    /// Returns an error if the database operation fails.
    pub fn get_manual_models(
        &self,
        provider_id: &str,
    ) -> Result<Vec<ManualModelEntry>, ConfigStoreError> {
        let conn = self.acquire_lock()?;
        manual_models::get_manual_models(&conn, provider_id)
    }

    /// Adds a manual model for a provider.
    ///
    /// # Errors
    ///
    /// Returns an error if the database operation fails.
    pub fn add_manual_model(
        &self,
        provider_id: &str,
        model: &ManualModelEntry,
    ) -> Result<(), ConfigStoreError> {
        let conn = self.acquire_lock()?;
        manual_models::add_manual_model(&conn, provider_id, model)
    }

    /// Removes a manual model for a provider.
    ///
    /// # Errors
    ///
    /// Returns an error if the database operation fails.
    pub fn remove_manual_model(
        &self,
        provider_id: &str,
        model_name: &str,
    ) -> Result<(), ConfigStoreError> {
        let conn = self.acquire_lock()?;
        manual_models::remove_manual_model(&conn, provider_id, model_name)
    }

    /// Sets the models source mode for a provider.
    ///
    /// # Errors
    ///
    /// Returns an error if the database operation fails.
    pub fn set_provider_models_source(
        &self,
        provider_id: &str,
        models_source: ModelsSource,
    ) -> Result<(), ConfigStoreError> {
        let conn = self.acquire_lock()?;
        providers::set_provider_models_source(&conn, provider_id, models_source)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_store_creation() {
        let store = ConfigStore::in_memory().expect("Failed to create store");
        let providers = store.get_providers().expect("Failed to get providers");
        assert_eq!(providers.len(), 3); // Built-in providers
    }

    #[test]
    fn test_save_and_get_provider() {
        let store = ConfigStore::in_memory().expect("Failed to create store");
        let provider = Provider::custom(
            "Test Provider",
            "TP",
            ApiType::OpenAI,
            Some("https://api.test.com".to_string()),
        );
        store
            .save_provider(&provider)
            .expect("Failed to save provider");

        let retrieved = store
            .get_provider(&provider.id)
            .expect("Failed to get provider");
        assert!(retrieved.is_some());
        let retrieved = retrieved.unwrap();
        assert_eq!(retrieved.name, "Test Provider");
        assert!(retrieved.is_custom);
    }

    #[test]
    fn test_delete_provider() {
        let store = ConfigStore::in_memory().expect("Failed to create store");
        let provider = Provider::custom("ToDelete", "TD", ApiType::OpenAI, None);
        store
            .save_provider(&provider)
            .expect("Failed to save provider");

        store
            .delete_provider(&provider.id)
            .expect("Failed to delete provider");

        let retrieved = store
            .get_provider(&provider.id)
            .expect("Failed to get provider");
        assert!(retrieved.is_none());
    }

    #[test]
    fn test_save_and_get_models() {
        let store = ConfigStore::in_memory().expect("Failed to create store");
        let models = vec![
            Model::new("openai", "gpt-4", ModelCapabilities::default()),
            Model::new("openai", "gpt-3.5-turbo", ModelCapabilities::default()),
        ];

        store
            .save_models("openai", &models)
            .expect("Failed to save models");

        let retrieved = store.get_models("openai").expect("Failed to get models");
        assert_eq!(retrieved.len(), 2);
    }

    #[test]
    fn test_operation_defaults() {
        let store = ConfigStore::in_memory().expect("Failed to create store");

        // First save a model (required for foreign key constraint)
        let model = Model::new("openai", "gpt-4", ModelCapabilities::default());
        store
            .save_models("openai", &[model])
            .expect("Failed to save model");

        // Now set the operation default
        store
            .set_operation_default(OperationType::Code, "openai-gpt-4")
            .expect("Failed to set default");

        let defaults = store
            .get_operation_defaults()
            .expect("Failed to get defaults");
        assert_eq!(defaults.len(), 1);
        assert_eq!(defaults[0].operation, OperationType::Code);
        assert_eq!(defaults[0].model_id, Some("openai-gpt-4".to_string()));
    }

    #[test]
    fn test_store_and_retrieve_api_key() {
        let store = ConfigStore::in_memory().expect("Failed to create store");

        // Store API key for built-in provider
        store
            .store_api_key("openai", "sk-test-key-12345")
            .expect("Failed to store API key");

        // Retrieve it
        let key = store.retrieve_api_key("openai").expect("Failed to retrieve API key");
        assert_eq!(key, "sk-test-key-12345");
    }

    #[test]
    fn test_retrieve_nonexistent_api_key() {
        let store = ConfigStore::in_memory().expect("Failed to create store");

        let result = store.retrieve_api_key("nonexistent");
        assert!(result.is_err());
    }

    #[test]
    fn test_has_api_key() {
        let store = ConfigStore::in_memory().expect("Failed to create store");

        // Should not exist initially
        let exists = store.has_api_key("openai").expect("Failed to check API key");
        assert!(!exists);

        // Store the key
        store.store_api_key("openai", "sk-test-key").expect("Failed to store API key");

        // Should exist now
        let exists = store.has_api_key("openai").expect("Failed to check API key");
        assert!(exists);
    }

    #[test]
    fn test_delete_api_key() {
        let store = ConfigStore::in_memory().expect("Failed to create store");

        // Store and delete
        store.store_api_key("openai", "sk-test-key").expect("Failed to store API key");

        store.delete_api_key("openai").expect("Failed to delete API key");

        // Should not exist anymore
        let exists = store.has_api_key("openai").expect("Failed to check API key");
        assert!(!exists);
    }

    #[test]
    fn test_update_api_key() {
        let store = ConfigStore::in_memory().expect("Failed to create store");

        // Store initial key
        store.store_api_key("openai", "sk-old-key").expect("Failed to store API key");

        // Update with new key
        store.store_api_key("openai", "sk-new-key").expect("Failed to update API key");

        // Should have new key
        let key = store.retrieve_api_key("openai").expect("Failed to retrieve API key");
        assert_eq!(key, "sk-new-key");
    }
}

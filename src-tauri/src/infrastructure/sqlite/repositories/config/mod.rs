//! `SQLite` storage for LLM provider configuration.
//!
//! This module provides persistence for providers, models, and operation defaults
//! using `SQLite`.

use std::sync::{Mutex, MutexGuard};

use rusqlite::Connection;
use thiserror::Error;

use crate::infrastructure::sqlite::migrations::{config, config_schema};

use crate::core::config::{
    ApiKeyRepository, CachedWebSearchEntry, ConfigDomainError, ManualModelEntry,
    ManualModelRepository, Model, ModelRepository, ModelsSource, NewCachedWebSearchEntry,
    OperationDefault, OperationDefaultsRepository, OperationType, PdfThemeRepository, Provider,
    ProviderRepository, StoredWorkspaceSearchPolicy, ToolCredentialsRepository,
    WebSearchCacheRepository, WorkspaceSearchPolicyRepository,
};

// Re-export submodule functions
pub(crate) mod api_keys;
pub mod helpers;
pub(crate) mod manual_models;
pub(crate) mod models;
pub(crate) mod operation_defaults;
pub(crate) mod providers;
pub(crate) mod tool_credentials;
pub(crate) mod pdf_theme;
pub(crate) mod web_search_cache;
pub(crate) mod workspace_search_policy;

/// Errors from the `SQLite` configuration repository.
#[derive(Debug, Error)]
pub enum ConfigRepositoryError {
    /// `SQLite` error
    #[error("SQLite error: {0}")]
    Sqlite(#[from] rusqlite::Error),
    /// Not found error
    #[error("{0} not found: {1}")]
    NotFound(&'static str, String),
    /// Lock error
    #[error("Lock error: {0}")]
    LockError(String),
}

/// SQLite-backed configuration repository.
pub struct SqliteConfigRepository {
    /// Database connection (protected by mutex for thread safety)
    conn: Mutex<Connection>,
}

impl SqliteConfigRepository {
    /// Creates a new configuration repository at the given path.
    ///
    /// # Errors
    ///
    /// Returns an error if the database cannot be opened or initialized.
    pub fn new(db_path: &str) -> Result<Self, ConfigRepositoryError> {
        let conn = Connection::open(db_path)?;
        config_schema::init_schema(&conn)?;
        config::migrate_api_type_constraint(&conn)?;
        config::migrate_models_source(&conn)?;
        config::normalize_base_urls(&conn)?;
        let store = Self {
            conn: Mutex::new(conn),
        };
        store.seed_builtin_providers()?;
        Ok(store)
    }

    /// Creates an in-memory configuration repository (for testing).
    ///
    /// # Errors
    ///
    /// Returns an error if the database cannot be initialized.
    pub fn in_memory() -> Result<Self, ConfigRepositoryError> {
        let conn = Connection::open_in_memory()?;
        config_schema::init_schema(&conn)?;
        config::migrate_api_type_constraint(&conn)?;
        config::migrate_models_source(&conn)?;
        config::normalize_base_urls(&conn)?;
        let store = Self {
            conn: Mutex::new(conn),
        };
        store.seed_builtin_providers()?;
        Ok(store)
    }

    fn acquire_lock(&self) -> Result<MutexGuard<'_, Connection>, ConfigRepositoryError> {
        self.conn
            .lock()
            .map_err(|_| ConfigRepositoryError::LockError("Failed to acquire lock".to_string()))
    }

    fn seed_builtin_providers(&self) -> Result<(), ConfigRepositoryError> {
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
    pub fn get_providers(&self) -> Result<Vec<Provider>, ConfigRepositoryError> {
        let conn = self.acquire_lock()?;
        providers::get_providers(&conn)
    }

    /// Gets a provider by ID.
    ///
    /// # Errors
    ///
    /// Returns an error if the database operation fails.
    pub fn get_provider(&self, id: &str) -> Result<Option<Provider>, ConfigRepositoryError> {
        let conn = self.acquire_lock()?;
        providers::get_provider(&conn, id)
    }

    /// Saves a provider (create or update).
    ///
    /// # Errors
    ///
    /// Returns an error if the database operation fails.
    pub fn save_provider(&self, provider: &Provider) -> Result<(), ConfigRepositoryError> {
        let conn = self.acquire_lock()?;
        providers::save_provider(&conn, provider)
    }

    /// Deletes a provider.
    ///
    /// # Errors
    ///
    /// Returns an error if the database operation fails.
    pub fn delete_provider(&self, id: &str) -> Result<(), ConfigRepositoryError> {
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
    ) -> Result<(), ConfigRepositoryError> {
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
    pub fn get_models(&self, provider_id: &str) -> Result<Vec<Model>, ConfigRepositoryError> {
        let conn = self.acquire_lock()?;
        models::get_models(&conn, provider_id)
    }

    /// Saves models for a provider (replaces existing).
    ///
    /// # Errors
    ///
    /// Returns an error if the database operation fails.
    pub fn save_models(
        &self,
        provider_id: &str,
        models: &[Model],
    ) -> Result<(), ConfigRepositoryError> {
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
    pub fn add_model(&self, model: &Model) -> Result<(), ConfigRepositoryError> {
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
    ) -> Result<(), ConfigRepositoryError> {
        let conn = self.acquire_lock()?;
        models::delete_model_by_name(&conn, provider_id, model_name)
    }

    /// Gets a model by ID.
    ///
    /// # Errors
    ///
    /// Returns an error if the database operation fails.
    pub fn get_model(&self, id: &str) -> Result<Option<Model>, ConfigRepositoryError> {
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
    ) -> Result<Option<Model>, ConfigRepositoryError> {
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
    pub fn get_operation_defaults(&self) -> Result<Vec<OperationDefault>, ConfigRepositoryError> {
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
    ) -> Result<(), ConfigRepositoryError> {
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
    ) -> Result<(), ConfigRepositoryError> {
        let conn = self.acquire_lock()?;
        operation_defaults::clear_operation_default(&conn, operation)
    }

    /// Clears all operation defaults for models belonging to a provider.
    ///
    /// # Errors
    ///
    /// Returns an error if the database operation fails.
    pub fn clear_defaults_for_provider(
        &self,
        provider_id: &str,
    ) -> Result<(), ConfigRepositoryError> {
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
    pub fn store_api_key(
        &self,
        provider_id: &str,
        api_key: &str,
    ) -> Result<(), ConfigRepositoryError> {
        let conn = self.acquire_lock()?;
        api_keys::store_api_key(&conn, provider_id, api_key)
    }

    /// Retrieves an API key for a provider.
    ///
    /// # Errors
    ///
    /// Returns an error if the key is not found or database operation fails.
    pub fn retrieve_api_key(&self, provider_id: &str) -> Result<String, ConfigRepositoryError> {
        let conn = self.acquire_lock()?;
        api_keys::retrieve_api_key(&conn, provider_id)
    }

    /// Deletes an API key for a provider.
    ///
    /// # Errors
    ///
    /// Returns an error if the database operation fails.
    pub fn delete_api_key(&self, provider_id: &str) -> Result<(), ConfigRepositoryError> {
        let conn = self.acquire_lock()?;
        api_keys::delete_api_key(&conn, provider_id)
    }

    /// Checks if an API key exists for a provider.
    ///
    /// # Errors
    ///
    /// Returns an error if the database operation fails.
    pub fn has_api_key(&self, provider_id: &str) -> Result<bool, ConfigRepositoryError> {
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
    ) -> Result<Vec<ManualModelEntry>, ConfigRepositoryError> {
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
    ) -> Result<(), ConfigRepositoryError> {
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
    ) -> Result<(), ConfigRepositoryError> {
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
    ) -> Result<(), ConfigRepositoryError> {
        let conn = self.acquire_lock()?;
        providers::set_provider_models_source(&conn, provider_id, models_source)
    }

    // =========================================================================
    // Tool Credentials Operations
    // =========================================================================

    /// Upserts the credentials blob for `tool_id`.
    ///
    /// # Errors
    ///
    /// Returns an error if the database operation fails.
    pub fn set_tool_credentials(
        &self,
        tool_id: &str,
        credentials: &str,
    ) -> Result<(), ConfigRepositoryError> {
        let conn = self.acquire_lock()?;
        tool_credentials::set_tool_credentials(&conn, tool_id, credentials)
    }

    /// Retrieves the credentials blob for `tool_id`, if present.
    ///
    /// # Errors
    ///
    /// Returns an error if the database operation fails.
    pub fn get_tool_credentials(
        &self,
        tool_id: &str,
    ) -> Result<Option<String>, ConfigRepositoryError> {
        let conn = self.acquire_lock()?;
        tool_credentials::get_tool_credentials(&conn, tool_id)
    }

    /// Deletes the credentials row for `tool_id` (no-op when absent).
    ///
    /// # Errors
    ///
    /// Returns an error if the database operation fails.
    pub fn delete_tool_credentials(&self, tool_id: &str) -> Result<(), ConfigRepositoryError> {
        let conn = self.acquire_lock()?;
        tool_credentials::delete_tool_credentials(&conn, tool_id)
    }

    /// Returns whether a credentials row exists for `tool_id`.
    ///
    /// # Errors
    ///
    /// Returns an error if the database operation fails.
    pub fn has_tool_credentials(&self, tool_id: &str) -> Result<bool, ConfigRepositoryError> {
        let conn = self.acquire_lock()?;
        tool_credentials::has_tool_credentials(&conn, tool_id)
    }

    // =========================================================================
    // Workspace Search Policy Operations
    // =========================================================================

    /// Reads the stored policy for `workspace_path`, if any.
    ///
    /// # Errors
    ///
    /// Returns an error if the database operation fails.
    pub fn get_workspace_search_policy(
        &self,
        workspace_path: &str,
    ) -> Result<Option<StoredWorkspaceSearchPolicy>, ConfigRepositoryError> {
        let conn = self.acquire_lock()?;
        Ok(workspace_search_policy::get_workspace_policy(
            &conn,
            workspace_path,
        )?
        .map(|p| StoredWorkspaceSearchPolicy {
            allowlist: p.allowlist,
            denylist: p.denylist,
        }))
    }

    /// Upserts the policy for `workspace_path`.
    ///
    /// # Errors
    ///
    /// Returns an error if the database operation fails.
    pub fn set_workspace_search_policy(
        &self,
        workspace_path: &str,
        allowlist: &[String],
        denylist: &[String],
    ) -> Result<(), ConfigRepositoryError> {
        let conn = self.acquire_lock()?;
        workspace_search_policy::set_workspace_policy(&conn, workspace_path, allowlist, denylist)
    }

    /// Deletes the stored policy for `workspace_path` (no-op when absent).
    ///
    /// # Errors
    ///
    /// Returns an error if the database operation fails.
    pub fn delete_workspace_search_policy(
        &self,
        workspace_path: &str,
    ) -> Result<(), ConfigRepositoryError> {
        let conn = self.acquire_lock()?;
        workspace_search_policy::delete_workspace_policy(&conn, workspace_path)
    }

    // =========================================================================
    // Web Search Cache Operations
    // =========================================================================

    /// Looks up a fresh cache entry by key.
    ///
    /// # Errors
    ///
    /// Returns an error if the database operation fails.
    pub fn get_fresh_web_search_cache(
        &self,
        cache_key: &str,
        now: chrono::DateTime<chrono::Utc>,
    ) -> Result<Option<CachedWebSearchEntry>, ConfigRepositoryError> {
        let conn = self.acquire_lock()?;
        Ok(web_search_cache::get_fresh(&conn, cache_key, now)?.map(|e| {
            CachedWebSearchEntry {
                results_json: e.results_json,
                provider: e.provider,
            }
        }))
    }

    /// Persists a new cache entry.
    ///
    /// # Errors
    ///
    /// Returns an error if the database operation fails.
    pub fn put_web_search_cache(
        &self,
        entry: &NewCachedWebSearchEntry<'_>,
    ) -> Result<(), ConfigRepositoryError> {
        let conn = self.acquire_lock()?;
        let raw = web_search_cache::NewCacheEntry {
            cache_key: entry.cache_key,
            query: entry.query,
            num_results: entry.num_results,
            language: entry.language,
            site: entry.site,
            workspace_path: entry.workspace_path,
            provider: entry.provider,
            results_json: entry.results_json,
            created_at: entry.created_at,
            expires_at: entry.expires_at,
        };
        web_search_cache::put(&conn, &raw)
    }

    /// Drops every expired cache row.
    ///
    /// # Errors
    ///
    /// Returns an error if the database operation fails.
    pub fn sweep_expired_web_search_cache(
        &self,
        now: chrono::DateTime<chrono::Utc>,
    ) -> Result<usize, ConfigRepositoryError> {
        let conn = self.acquire_lock()?;
        web_search_cache::sweep_expired(&conn, now)
    }

    /// Clears cache entries. `None` clears everything; `Some(path)` clears
    /// one workspace partition.
    ///
    /// # Errors
    ///
    /// Returns an error if the database operation fails.
    pub fn clear_web_search_cache(
        &self,
        workspace_path: Option<&str>,
    ) -> Result<usize, ConfigRepositoryError> {
        let conn = self.acquire_lock()?;
        web_search_cache::clear(&conn, workspace_path)
    }

    /// Total cache row count.
    ///
    /// # Errors
    ///
    /// Returns an error if the database operation fails.
    pub fn count_web_search_cache(&self) -> Result<usize, ConfigRepositoryError> {
        let conn = self.acquire_lock()?;
        web_search_cache::count(&conn)
    }
}

impl From<ConfigRepositoryError> for ConfigDomainError {
    fn from(err: ConfigRepositoryError) -> Self {
        match err {
            ConfigRepositoryError::NotFound(entity, id) => Self::NotFound { entity, id },
            ConfigRepositoryError::Sqlite(e) => Self::InvalidInput {
                reason: format!("Database error: {e}"),
            },
            ConfigRepositoryError::LockError(e) => Self::InvalidInput {
                reason: format!("Lock error: {e}"),
            },
        }
    }
}

#[allow(dead_code)]
#[allow(dead_code)]
impl ProviderRepository for SqliteConfigRepository {
    fn get_providers(&self) -> crate::core::config::ConfigRepositoryResult<Vec<Provider>> {
        Self::get_providers(self).map_err(Into::into)
    }

    fn get_provider(
        &self,
        id: &str,
    ) -> crate::core::config::ConfigRepositoryResult<Option<Provider>> {
        Self::get_provider(self, id).map_err(Into::into)
    }

    fn save_provider(
        &self,
        provider: &Provider,
    ) -> crate::core::config::ConfigRepositoryResult<()> {
        Self::save_provider(self, provider).map_err(Into::into)
    }

    fn delete_provider(&self, id: &str) -> crate::core::config::ConfigRepositoryResult<()> {
        Self::delete_provider(self, id).map_err(Into::into)
    }

    fn set_provider_configured(
        &self,
        id: &str,
        configured: bool,
    ) -> crate::core::config::ConfigRepositoryResult<()> {
        Self::set_provider_configured(self, id, configured).map_err(Into::into)
    }

    fn set_provider_models_source(
        &self,
        id: &str,
        models_source: ModelsSource,
    ) -> crate::core::config::ConfigRepositoryResult<()> {
        Self::set_provider_models_source(self, id, models_source).map_err(Into::into)
    }
}

impl ModelRepository for SqliteConfigRepository {
    fn get_models(
        &self,
        provider_id: &str,
    ) -> crate::core::config::ConfigRepositoryResult<Vec<Model>> {
        Self::get_models(self, provider_id).map_err(Into::into)
    }

    fn save_models(
        &self,
        provider_id: &str,
        models: &[Model],
    ) -> crate::core::config::ConfigRepositoryResult<()> {
        Self::save_models(self, provider_id, models).map_err(Into::into)
    }

    fn add_model(&self, model: &Model) -> crate::core::config::ConfigRepositoryResult<()> {
        Self::add_model(self, model).map_err(Into::into)
    }

    fn delete_model_by_name(
        &self,
        provider_id: &str,
        model_name: &str,
    ) -> crate::core::config::ConfigRepositoryResult<()> {
        Self::delete_model_by_name(self, provider_id, model_name).map_err(Into::into)
    }

    fn get_model(&self, id: &str) -> crate::core::config::ConfigRepositoryResult<Option<Model>> {
        Self::get_model(self, id).map_err(Into::into)
    }

    fn get_model_by_name(
        &self,
        provider_id: &str,
        name: &str,
    ) -> crate::core::config::ConfigRepositoryResult<Option<Model>> {
        Self::get_model_by_name(self, provider_id, name).map_err(Into::into)
    }
}

impl OperationDefaultsRepository for SqliteConfigRepository {
    fn get_operation_defaults(
        &self,
    ) -> crate::core::config::ConfigRepositoryResult<Vec<OperationDefault>> {
        Self::get_operation_defaults(self).map_err(Into::into)
    }

    fn set_operation_default(
        &self,
        operation: OperationType,
        model_id: &str,
    ) -> crate::core::config::ConfigRepositoryResult<()> {
        Self::set_operation_default(self, operation, model_id).map_err(Into::into)
    }

    fn clear_operation_default(
        &self,
        operation: OperationType,
    ) -> crate::core::config::ConfigRepositoryResult<()> {
        Self::clear_operation_default(self, operation).map_err(Into::into)
    }

    fn clear_defaults_for_provider(
        &self,
        provider_id: &str,
    ) -> crate::core::config::ConfigRepositoryResult<()> {
        Self::clear_defaults_for_provider(self, provider_id).map_err(Into::into)
    }
}

impl ApiKeyRepository for SqliteConfigRepository {
    fn store_api_key(
        &self,
        provider_id: &str,
        api_key: &str,
    ) -> crate::core::config::ConfigRepositoryResult<()> {
        Self::store_api_key(self, provider_id, api_key).map_err(Into::into)
    }

    fn retrieve_api_key(
        &self,
        provider_id: &str,
    ) -> crate::core::config::ConfigRepositoryResult<String> {
        Self::retrieve_api_key(self, provider_id).map_err(Into::into)
    }

    fn delete_api_key(&self, provider_id: &str) -> crate::core::config::ConfigRepositoryResult<()> {
        Self::delete_api_key(self, provider_id).map_err(Into::into)
    }

    fn has_api_key(&self, provider_id: &str) -> crate::core::config::ConfigRepositoryResult<bool> {
        Self::has_api_key(self, provider_id).map_err(Into::into)
    }
}

impl ManualModelRepository for SqliteConfigRepository {
    fn get_manual_models(
        &self,
        provider_id: &str,
    ) -> crate::core::config::ConfigRepositoryResult<Vec<ManualModelEntry>> {
        Self::get_manual_models(self, provider_id).map_err(Into::into)
    }

    fn add_manual_model(
        &self,
        provider_id: &str,
        entry: &ManualModelEntry,
    ) -> crate::core::config::ConfigRepositoryResult<()> {
        Self::add_manual_model(self, provider_id, entry).map_err(Into::into)
    }

    fn remove_manual_model(
        &self,
        provider_id: &str,
        name: &str,
    ) -> crate::core::config::ConfigRepositoryResult<()> {
        Self::remove_manual_model(self, provider_id, name).map_err(Into::into)
    }
}

impl ToolCredentialsRepository for SqliteConfigRepository {
    fn set_tool_credentials(
        &self,
        tool_id: &str,
        credentials: &str,
    ) -> crate::core::config::ConfigRepositoryResult<()> {
        Self::set_tool_credentials(self, tool_id, credentials).map_err(Into::into)
    }

    fn get_tool_credentials(
        &self,
        tool_id: &str,
    ) -> crate::core::config::ConfigRepositoryResult<Option<String>> {
        Self::get_tool_credentials(self, tool_id).map_err(Into::into)
    }

    fn delete_tool_credentials(
        &self,
        tool_id: &str,
    ) -> crate::core::config::ConfigRepositoryResult<()> {
        Self::delete_tool_credentials(self, tool_id).map_err(Into::into)
    }

    fn has_tool_credentials(
        &self,
        tool_id: &str,
    ) -> crate::core::config::ConfigRepositoryResult<bool> {
        Self::has_tool_credentials(self, tool_id).map_err(Into::into)
    }
}

impl WorkspaceSearchPolicyRepository for SqliteConfigRepository {
    fn get_workspace_search_policy(
        &self,
        workspace_path: &str,
    ) -> crate::core::config::ConfigRepositoryResult<Option<StoredWorkspaceSearchPolicy>> {
        Self::get_workspace_search_policy(self, workspace_path).map_err(Into::into)
    }

    fn set_workspace_search_policy(
        &self,
        workspace_path: &str,
        allowlist: &[String],
        denylist: &[String],
    ) -> crate::core::config::ConfigRepositoryResult<()> {
        Self::set_workspace_search_policy(self, workspace_path, allowlist, denylist)
            .map_err(Into::into)
    }

    fn delete_workspace_search_policy(
        &self,
        workspace_path: &str,
    ) -> crate::core::config::ConfigRepositoryResult<()> {
        Self::delete_workspace_search_policy(self, workspace_path).map_err(Into::into)
    }
}

impl WebSearchCacheRepository for SqliteConfigRepository {
    fn get_fresh(
        &self,
        cache_key: &str,
        now: chrono::DateTime<chrono::Utc>,
    ) -> crate::core::config::ConfigRepositoryResult<Option<CachedWebSearchEntry>> {
        Self::get_fresh_web_search_cache(self, cache_key, now).map_err(Into::into)
    }

    fn put(
        &self,
        entry: &NewCachedWebSearchEntry<'_>,
    ) -> crate::core::config::ConfigRepositoryResult<()> {
        Self::put_web_search_cache(self, entry).map_err(Into::into)
    }

    fn sweep_expired(
        &self,
        now: chrono::DateTime<chrono::Utc>,
    ) -> crate::core::config::ConfigRepositoryResult<usize> {
        Self::sweep_expired_web_search_cache(self, now).map_err(Into::into)
    }

    fn clear(
        &self,
        workspace_path: Option<&str>,
    ) -> crate::core::config::ConfigRepositoryResult<usize> {
        Self::clear_web_search_cache(self, workspace_path).map_err(Into::into)
    }

    fn count(&self) -> crate::core::config::ConfigRepositoryResult<usize> {
        Self::count_web_search_cache(self).map_err(Into::into)
    }
}

impl PdfThemeRepository for SqliteConfigRepository {
    fn get_pdf_theme(
        &self,
        workspace_path: &str,
    ) -> crate::core::config::ConfigRepositoryResult<Option<String>> {
        let conn = self.acquire_lock()?;
        pdf_theme::get_pdf_theme(&conn, workspace_path).map_err(Into::into)
    }

    fn set_pdf_theme(
        &self,
        workspace_path: &str,
        theme_name: &str,
    ) -> crate::core::config::ConfigRepositoryResult<()> {
        let conn = self.acquire_lock()?;
        pdf_theme::set_pdf_theme(&conn, workspace_path, theme_name).map_err(Into::into)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::config::{ApiType, ModelCapabilities};

    #[test]
    fn test_store_creation() {
        let store = SqliteConfigRepository::in_memory().expect("Failed to create store");
        let providers = store.get_providers().expect("Failed to get providers");
        assert_eq!(providers.len(), 3); // Built-in providers
    }

    #[test]
    fn test_save_and_get_provider() {
        let store = SqliteConfigRepository::in_memory().expect("Failed to create store");
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
        let store = SqliteConfigRepository::in_memory().expect("Failed to create store");
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
        let store = SqliteConfigRepository::in_memory().expect("Failed to create store");
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
        let store = SqliteConfigRepository::in_memory().expect("Failed to create store");

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
        let store = SqliteConfigRepository::in_memory().expect("Failed to create store");

        // Persist API key for built-in provider
        store
            .store_api_key("openai", "sk-test-key-12345")
            .expect("Failed to store API key");

        // Retrieve it
        let key = store
            .retrieve_api_key("openai")
            .expect("Failed to retrieve API key");
        assert_eq!(key, "sk-test-key-12345");
    }

    #[test]
    fn test_retrieve_nonexistent_api_key() {
        let store = SqliteConfigRepository::in_memory().expect("Failed to create store");

        let result = store.retrieve_api_key("nonexistent");
        assert!(result.is_err());
    }

    #[test]
    fn test_has_api_key() {
        let store = SqliteConfigRepository::in_memory().expect("Failed to create store");

        // Should not exist initially
        let exists = store
            .has_api_key("openai")
            .expect("Failed to check API key");
        assert!(!exists);

        // Persist the key
        store
            .store_api_key("openai", "sk-test-key")
            .expect("Failed to store API key");

        // Should exist now
        let exists = store
            .has_api_key("openai")
            .expect("Failed to check API key");
        assert!(exists);
    }

    #[test]
    fn test_delete_api_key() {
        let store = SqliteConfigRepository::in_memory().expect("Failed to create store");

        // Persist and delete
        store
            .store_api_key("openai", "sk-test-key")
            .expect("Failed to store API key");

        store
            .delete_api_key("openai")
            .expect("Failed to delete API key");

        // Should not exist anymore
        let exists = store
            .has_api_key("openai")
            .expect("Failed to check API key");
        assert!(!exists);
    }

    #[test]
    fn test_update_api_key() {
        let store = SqliteConfigRepository::in_memory().expect("Failed to create store");

        // Persist initial key
        store
            .store_api_key("openai", "sk-old-key")
            .expect("Failed to store API key");

        // Update with new key
        store
            .store_api_key("openai", "sk-new-key")
            .expect("Failed to update API key");

        // Should have new key
        let key = store
            .retrieve_api_key("openai")
            .expect("Failed to retrieve API key");
        assert_eq!(key, "sk-new-key");
    }
}

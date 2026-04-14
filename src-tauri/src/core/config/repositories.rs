//! Defines the repository interfaces for configuration data.
//!
//! This module provides traits that abstract the persistence of provider, model,
//! API key, and operation default configurations used throughout the backend.
//! These interfaces decouple the core logic from specific storage implementations.

use thiserror::Error;

use super::{ManualModelEntry, Model, ModelsSource, OperationDefault, OperationType, Provider};

/// A specialized `Result` type for configuration repository operations.
pub type RepositoryResult<T> = Result<T, ConfigDomainError>;

/// Represents a domain-level error that can occur during configuration operations.
#[derive(Debug, Clone, Error)]
pub enum ConfigDomainError {
    /// Indicates that a requested record was not found in the repository.
    #[error("{entity} not found: {id}")]
    NotFound {
        /// The type of the entity that was not found.
        entity: &'static str,
        /// The ID of the entity that was not found.
        id: String,
    },
    /// Indicates that the provided input or persisted state is invalid.
    #[error("Invalid configuration: {reason}")]
    InvalidInput {
        /// A description of the reason for the invalid input.
        reason: String,
    },
}

/// A repository for managing `Provider` configurations.
pub trait ProviderRepository: Send + Sync {
    /// Retrieves all provider configurations.
    fn get_providers(&self) -> RepositoryResult<Vec<Provider>>;
    /// Retrieves a provider configuration by its unique ID.
    fn get_provider(&self, id: &str) -> RepositoryResult<Option<Provider>>;
    /// Saves a provider configuration.
    fn save_provider(&self, provider: &Provider) -> RepositoryResult<()>;
    /// Deletes a provider configuration by its unique ID.
    fn delete_provider(&self, id: &str) -> RepositoryResult<()>;
    /// Sets the configured status for a provider.
    fn set_provider_configured(&self, id: &str, configured: bool) -> RepositoryResult<()>;
    /// Sets the model source for a provider.
    fn set_provider_models_source(
        &self,
        id: &str,
        models_source: ModelsSource,
    ) -> RepositoryResult<()>;
}

/// A repository for managing `Model` persistence.
pub trait ModelRepository: Send + Sync {
    /// Retrieves all models for a specific provider.
    fn get_models(&self, provider_id: &str) -> RepositoryResult<Vec<Model>>;
    /// Saves a list of models for a specific provider.
    fn save_models(&self, provider_id: &str, models: &[Model]) -> RepositoryResult<()>;
    /// Adds a new model to the repository.
    fn add_model(&self, model: &Model) -> RepositoryResult<()>;
    /// Deletes a model by its name for a specific provider.
    fn delete_model_by_name(&self, provider_id: &str, model_name: &str) -> RepositoryResult<()>;
    /// Retrieves a model by its unique ID.
    fn get_model(&self, id: &str) -> RepositoryResult<Option<Model>>;
    /// Retrieves a model by its name for a specific provider.
    fn get_model_by_name(&self, provider_id: &str, name: &str) -> RepositoryResult<Option<Model>>;
}

/// A repository for managing default model assignments for operations.
pub trait OperationDefaultsRepository: Send + Sync {
    /// Retrieves all operation defaults.
    fn get_operation_defaults(&self) -> RepositoryResult<Vec<OperationDefault>>;
    /// Sets the default model for a specific operation.
    fn set_operation_default(
        &self,
        operation: OperationType,
        model_id: &str,
    ) -> RepositoryResult<()>;
    /// Clears the default model for a specific operation.
    fn clear_operation_default(&self, operation: OperationType) -> RepositoryResult<()>;
    /// Clears all default assignments for a specific provider.
    fn clear_defaults_for_provider(&self, provider_id: &str) -> RepositoryResult<()>;
}

/// A repository for securely storing and retrieving provider API keys.
pub trait ApiKeyRepository: Send + Sync {
    /// Stores an API key for a specific provider.
    fn store_api_key(&self, provider_id: &str, api_key: &str) -> RepositoryResult<()>;
    /// Retrieves the API key for a specific provider.
    fn retrieve_api_key(&self, provider_id: &str) -> RepositoryResult<String>;
    /// Deletes the API key for a specific provider.
    fn delete_api_key(&self, provider_id: &str) -> RepositoryResult<()>;
    /// Checks if an API key exists for a specific provider.
    fn has_api_key(&self, provider_id: &str) -> RepositoryResult<bool>;
}

/// A repository for tool-owned credential blobs.
///
/// The value stored for `tool_id` is opaque JSON whose shape is owned by the
/// tool (e.g. the web-search tool stores `{"api_key": "…", "cx": "…"}`). This
/// repository does not interpret the blob — it only persists it.
pub trait ToolCredentialsRepository: Send + Sync {
    /// Upserts the credentials blob for `tool_id`.
    fn set_tool_credentials(&self, tool_id: &str, credentials: &str) -> RepositoryResult<()>;
    /// Retrieves the credentials blob for `tool_id`, if any.
    fn get_tool_credentials(&self, tool_id: &str) -> RepositoryResult<Option<String>>;
    /// Removes the stored credentials for `tool_id` (no-op when absent).
    fn delete_tool_credentials(&self, tool_id: &str) -> RepositoryResult<()>;
    /// Returns `true` when credentials are persisted for `tool_id`.
    fn has_tool_credentials(&self, tool_id: &str) -> RepositoryResult<bool>;
}

/// A repository for managing manual model overrides for a provider.
pub trait ManualModelRepository: Send + Sync {
    /// Retrieves all manual models for a specific provider.
    fn get_manual_models(&self, provider_id: &str) -> RepositoryResult<Vec<ManualModelEntry>>;
    /// Adds a new manual model entry for a provider.
    fn add_manual_model(&self, provider_id: &str, entry: &ManualModelEntry)
        -> RepositoryResult<()>;
    /// Removes a manual model entry by its name for a specific provider.
    fn remove_manual_model(&self, provider_id: &str, name: &str) -> RepositoryResult<()>;
}

/// A combined repository contract for all configuration-related persistence.
///
/// This trait serves as a convenient way to require all configuration repository
/// capabilities behind a single dependency, which is useful for adapters
/// that implement all the necessary traits.
pub trait ConfigRepository:
    ProviderRepository
    + ModelRepository
    + OperationDefaultsRepository
    + ApiKeyRepository
    + ManualModelRepository
    + ToolCredentialsRepository
{
}

// A blanket implementation that applies `ConfigRepository` to any type
// that implements all the required underlying repository traits.
impl<T> ConfigRepository for T where
    T: ProviderRepository
        + ModelRepository
        + OperationDefaultsRepository
        + ApiKeyRepository
        + ManualModelRepository
        + ToolCredentialsRepository
{
}

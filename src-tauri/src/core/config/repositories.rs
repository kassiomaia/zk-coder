//! Configuration repository interfaces.
//!
//! Defines persistence contracts for provider, model, API key, and operation
//! default configuration used across the backend.

use thiserror::Error;

use super::{ManualModelEntry, Model, ModelsSource, OperationDefault, OperationType, Provider};

/// Result type for configuration repository operations.
pub type RepositoryResult<T> = Result<T, ConfigDomainError>;

/// Domain-level configuration error.
#[derive(Debug, Clone, Error)]
pub enum ConfigDomainError {
    /// A requested record was not found.
    #[error("{entity} not found: {id}")]
    NotFound { entity: &'static str, id: String },
    /// The provided input or persisted state is invalid.
    #[error("Invalid configuration: {reason}")]
    InvalidInput { reason: String },
}

/// Repository for provider configuration.
pub trait ProviderRepository: Send + Sync {
    fn get_providers(&self) -> RepositoryResult<Vec<Provider>>;
    fn get_provider(&self, id: &str) -> RepositoryResult<Option<Provider>>;
    fn save_provider(&self, provider: &Provider) -> RepositoryResult<()>;
    fn delete_provider(&self, id: &str) -> RepositoryResult<()>;
    fn set_provider_configured(&self, id: &str, configured: bool) -> RepositoryResult<()>;
    fn set_provider_models_source(
        &self,
        id: &str,
        models_source: ModelsSource,
    ) -> RepositoryResult<()>;
}

/// Repository for model persistence.
pub trait ModelRepository: Send + Sync {
    fn get_models(&self, provider_id: &str) -> RepositoryResult<Vec<Model>>;
    fn save_models(&self, provider_id: &str, models: &[Model]) -> RepositoryResult<()>;
    fn add_model(&self, model: &Model) -> RepositoryResult<()>;
    fn delete_model_by_name(&self, provider_id: &str, model_name: &str) -> RepositoryResult<()>;
    fn get_model(&self, id: &str) -> RepositoryResult<Option<Model>>;
    fn get_model_by_name(&self, provider_id: &str, name: &str) -> RepositoryResult<Option<Model>>;
}

/// Repository for operation-default configuration.
pub trait OperationDefaultsRepository: Send + Sync {
    fn get_operation_defaults(&self) -> RepositoryResult<Vec<OperationDefault>>;
    fn set_operation_default(
        &self,
        operation: OperationType,
        model_id: &str,
    ) -> RepositoryResult<()>;
    fn clear_operation_default(&self, operation: OperationType) -> RepositoryResult<()>;
    fn clear_defaults_for_provider(&self, provider_id: &str) -> RepositoryResult<()>;
}

/// Repository for provider API key storage.
pub trait ApiKeyRepository: Send + Sync {
    fn store_api_key(&self, provider_id: &str, api_key: &str) -> RepositoryResult<()>;
    fn retrieve_api_key(&self, provider_id: &str) -> RepositoryResult<String>;
    fn delete_api_key(&self, provider_id: &str) -> RepositoryResult<()>;
    fn has_api_key(&self, provider_id: &str) -> RepositoryResult<bool>;
}

/// Repository for manual model overrides.
pub trait ManualModelRepository: Send + Sync {
    fn get_manual_models(&self, provider_id: &str) -> RepositoryResult<Vec<ManualModelEntry>>;
    fn add_manual_model(&self, provider_id: &str, entry: &ManualModelEntry)
        -> RepositoryResult<()>;
    fn remove_manual_model(&self, provider_id: &str, name: &str) -> RepositoryResult<()>;
}

/// Combined configuration repository contract.
pub trait ConfigRepository:
    ProviderRepository
    + ModelRepository
    + OperationDefaultsRepository
    + ApiKeyRepository
    + ManualModelRepository
{
}

impl<T> ConfigRepository for T where
    T: ProviderRepository
        + ModelRepository
        + OperationDefaultsRepository
        + ApiKeyRepository
        + ManualModelRepository
{
}

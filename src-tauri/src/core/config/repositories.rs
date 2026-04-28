//! Defines the repository interfaces for configuration data.
//!
//! This module provides traits that abstract the persistence of provider, model,
//! API key, and operation default configurations used throughout the backend.
//! These interfaces decouple the core logic from specific storage implementations.

use thiserror::Error;

use super::{
    ManualModelEntry, Model, ModelsSource, OperationDefault, OperationType, PromptTemplate,
    Provider,
};

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

/// Allow/deny domain lists as persisted for one workspace. Empty vectors mean
/// "no filter in that direction".
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct StoredWorkspaceSearchPolicy {
    /// Domains the tool is allowed to search (empty = any).
    pub allowlist: Vec<String>,
    /// Domains the tool must never search.
    pub denylist: Vec<String>,
}

/// A fresh cache hit for the web-search tool.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CachedWebSearchEntry {
    /// JSON-encoded `Vec<WebSearchResult>` as persisted. The tool layer
    /// deserializes this into the typed shape.
    pub results_json: String,
    /// Provider identifier recorded at insert time.
    pub provider: String,
}

/// New entry to be persisted in the web-search cache.
pub struct NewCachedWebSearchEntry<'a> {
    /// SHA-256-derived key owned by the tool layer.
    pub cache_key: &'a str,
    /// Normalized query string.
    pub query: &'a str,
    /// Number of results requested.
    pub num_results: usize,
    /// BCP-47 language tag, normalized to lowercase.
    pub language: Option<&'a str>,
    /// Site restriction, normalized to lowercase.
    pub site: Option<&'a str>,
    /// Canonical workspace path (empty string = global bucket).
    pub workspace_path: &'a str,
    /// Provider identifier.
    pub provider: &'a str,
    /// JSON-encoded results payload.
    pub results_json: &'a str,
    /// Timestamp at which this entry was created.
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// Timestamp at which this entry should be treated as expired.
    pub expires_at: chrono::DateTime<chrono::Utc>,
}

/// A repository for the web-search tool's result cache.
pub trait WebSearchCacheRepository: Send + Sync {
    /// Returns the cached entry for `cache_key` when it exists and is still
    /// fresh relative to `now`.
    fn get_fresh(
        &self,
        cache_key: &str,
        now: chrono::DateTime<chrono::Utc>,
    ) -> RepositoryResult<Option<CachedWebSearchEntry>>;
    /// Upserts a cache entry.
    fn put(&self, entry: &NewCachedWebSearchEntry<'_>) -> RepositoryResult<()>;
    /// Deletes every row whose `expires_at <= now`. Returns the number removed.
    fn sweep_expired(&self, now: chrono::DateTime<chrono::Utc>) -> RepositoryResult<usize>;
    /// `Some(path)` clears only that workspace partition; `None` clears everything.
    fn clear(&self, workspace_path: Option<&str>) -> RepositoryResult<usize>;
    /// Total row count including not-yet-swept expired entries.
    fn count(&self) -> RepositoryResult<usize>;
}

/// A repository for the web-search tool's per-workspace domain policy.
///
/// The `workspace_path` is the canonicalized absolute path of the workspace
/// root; the empty string is a sentinel for "no workspace" (global fallback).
/// Entries not present in the store are treated as an empty policy (no filter).
pub trait WorkspaceSearchPolicyRepository: Send + Sync {
    /// Retrieves the policy for `workspace_path`, or `None` when absent.
    fn get_workspace_search_policy(
        &self,
        workspace_path: &str,
    ) -> RepositoryResult<Option<StoredWorkspaceSearchPolicy>>;
    /// Upserts the policy for `workspace_path`.
    fn set_workspace_search_policy(
        &self,
        workspace_path: &str,
        allowlist: &[String],
        denylist: &[String],
    ) -> RepositoryResult<()>;
    /// Deletes the stored policy for `workspace_path` (no-op when absent).
    fn delete_workspace_search_policy(&self, workspace_path: &str) -> RepositoryResult<()>;
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
/// PDF theme selection persistence (T3.3).
pub trait PdfThemeRepository: Send + Sync {
    /// Returns the selected theme name for a workspace, or `None` for
    /// the default.
    fn get_pdf_theme(&self, workspace_path: &str) -> RepositoryResult<Option<String>>;
    /// Persists the selected theme for a workspace.
    fn set_pdf_theme(&self, workspace_path: &str, theme_name: &str) -> RepositoryResult<()>;
}

/// UI theme preference persistence (global, single-row).
///
/// The stored value is one of `light`, `dark`, or `system`. `None` means the
/// user has not set a preference yet, and the frontend should fall back to the
/// operating system's preferred color scheme.
pub trait UiThemeRepository: Send + Sync {
    /// Returns the persisted UI theme, or `None` when no preference is set.
    fn get_ui_theme(&self) -> RepositoryResult<Option<String>>;
    /// Persists the UI theme. Implementations enforce the allowed value set.
    fn set_ui_theme(&self, theme: &str) -> RepositoryResult<()>;
}

/// A repository for prompt templates (slash-command expansions).
pub trait PromptTemplateRepository: Send + Sync {
    /// Returns all templates ordered by `sort_order`, then `slug`.
    fn list_prompt_templates(&self) -> RepositoryResult<Vec<PromptTemplate>>;
    /// Looks up a template by slug, or `None` when absent.
    fn get_prompt_template_by_slug(
        &self,
        slug: &str,
    ) -> RepositoryResult<Option<PromptTemplate>>;
    /// Creates or updates a template. Returns the persisted row.
    fn upsert_prompt_template(
        &self,
        template: &PromptTemplate,
    ) -> RepositoryResult<PromptTemplate>;
    /// Deletes a template by id. Builtins are not deletable; callers should
    /// use `reset_prompt_template` instead.
    fn delete_prompt_template(&self, id: &str) -> RepositoryResult<()>;
    /// Restores a builtin template's row to its seeded factory default.
    /// Returns the refreshed row. Fails for non-builtin slugs.
    fn reset_prompt_template(&self, slug: &str) -> RepositoryResult<PromptTemplate>;
}

/// Combined configuration repository trait that aggregates all config-related
/// repository capabilities (providers, models, API keys, tool credentials, etc.).
pub trait ConfigRepository:
    ProviderRepository
    + ModelRepository
    + OperationDefaultsRepository
    + ApiKeyRepository
    + ManualModelRepository
    + ToolCredentialsRepository
    + WorkspaceSearchPolicyRepository
    + WebSearchCacheRepository
    + PdfThemeRepository
    + UiThemeRepository
    + PromptTemplateRepository
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
        + WorkspaceSearchPolicyRepository
        + WebSearchCacheRepository
        + PdfThemeRepository
        + UiThemeRepository
        + PromptTemplateRepository
{
}

//! Configuration Tauri IPC commands.
//!
//! This module provides Tauri commands for LLM provider configuration management.

use std::sync::Arc;
use tokio::sync::Mutex;

use crate::api;
use crate::nlp::config::{
    ApiType, FetchModelsResult, ManualModelEntry, ManualModelRequest, Model, ModelCapabilities,
    ModelsSource, OperationDefault, OperationType, Provider, SaveProviderRequest,
    ValidationResult,
};
use crate::storage::ConfigStore;

// Re-export log macros for debugging
use log::{info, error};

/// Application state for configuration commands.
pub struct ConfigState {
    /// Config store for persistence
    pub store: Arc<Mutex<ConfigStore>>,
}

impl ConfigState {
    /// Creates a new config state.
    ///
    /// # Errors
    ///
    /// Returns an error if the store cannot be initialized.
    pub fn new(db_path: &str) -> Result<Self, String> {
        let store = ConfigStore::new(db_path).map_err(|e| e.to_string())?;
        Ok(Self {
            store: Arc::new(Mutex::new(store)),
        })
    }

    /// Creates a new config state with an in-memory database (for testing).
    ///
    /// # Errors
    ///
    /// Returns an error if the store cannot be initialized.
    pub fn in_memory() -> Result<Self, String> {
        let store = ConfigStore::in_memory().map_err(|e| e.to_string())?;
        Ok(Self {
            store: Arc::new(Mutex::new(store)),
        })
    }
}

// =========================================================================
// Provider Commands
// =========================================================================

/// Gets all providers.
#[tauri::command]
pub async fn get_providers(state: tauri::State<'_, ConfigState>) -> Result<Vec<Provider>, String> {
    info!("get_providers: Starting to fetch providers...");
    let store = state.store.lock().await;
    let providers = store.get_providers().map_err(|e| {
        error!("get_providers: Failed to get providers: {}", e);
        e.to_string()
    })?;
    info!("get_providers: Returning {} providers", providers.len());
    for p in &providers {
        info!(
            "get_providers: Provider id={}, name={}, is_custom={}, is_configured={}",
            p.id, p.name, p.is_custom, p.is_configured
        );
    }
    Ok(providers)
}

/// Built-in provider IDs that cannot be used for custom providers
const BUILTIN_PROVIDER_IDS: &[&str] = &["anthropic", "openai", "google"];

/// Saves a provider (create or update).
#[tauri::command]
pub async fn save_provider(
    request: SaveProviderRequest,
    state: tauri::State<'_, ConfigState>,
) -> Result<Provider, String> {
    let store = state.store.lock().await;

    let provider = if let Some(id) = request.id {
        // Check if this is an update or a create with specific ID
        let existing = store.get_provider(&id).map_err(|e| e.to_string())?;

        if let Some(existing) = existing {
            // Update existing provider (preserve is_custom and models_source for built-in providers)
            Provider {
                id,
                name: request.name,
                icon: request.icon,
                is_custom: existing.is_custom,
                api_type: request.api_type,
                base_url: request.base_url,
                is_configured: existing.is_configured,
                models_source: existing.models_source,
                created_at: existing.created_at,
                updated_at: chrono::Utc::now(),
            }
        } else {
            // Prevent creating custom providers with built-in IDs
            if BUILTIN_PROVIDER_IDS.contains(&id.as_str()) {
                return Err(format!(
                    "Cannot create custom provider with reserved ID '{}'",
                    id
                ));
            }
            // Create new custom provider with the specified ID
            Provider::custom_with_id(
                &id,
                &request.name,
                &request.icon,
                request.api_type,
                request.base_url,
            )
        }
    } else {
        // Create new custom provider with auto-generated ID
        Provider::custom(&request.name, &request.icon, request.api_type, request.base_url)
    };

    store.save_provider(&provider).map_err(|e| e.to_string())?;
    Ok(provider)
}

/// Removes a custom provider.
#[tauri::command]
pub async fn remove_provider(
    provider_id: String,
    state: tauri::State<'_, ConfigState>,
) -> Result<(), String> {
    let store = state.store.lock().await;

    // Delete API key from database (ignore errors if not found)
    let _ = store.delete_api_key(&provider_id);

    // Clear operation defaults for this provider's models
    store
        .clear_defaults_for_provider(&provider_id)
        .map_err(|e| e.to_string())?;

    // Delete provider from database (cascade deletes models)
    store
        .delete_provider(&provider_id)
        .map_err(|e| e.to_string())?;

    Ok(())
}

// =========================================================================
// API Key Commands
// =========================================================================

/// Validates an API key with the provider.
#[tauri::command]
pub async fn validate_api_key(
    _provider_id: String,
    api_key: String,
    api_type: ApiType,
    base_url: Option<String>,
) -> Result<ValidationResult, String> {
    // Empty API key check
    if api_key.trim().is_empty() {
        return Ok(ValidationResult::failure("Please enter an API key"));
    }

    api::validate_api_key(api_type, &api_key, base_url.as_deref()).await
}

/// Tests connection to a custom provider.
#[tauri::command]
pub async fn test_connection(
    base_url: String,
    api_key: String,
    api_type: ApiType,
) -> Result<ValidationResult, String> {
    // Empty API key check
    if api_key.trim().is_empty() {
        return Ok(ValidationResult::failure(
            "Please enter an API key first",
        ));
    }

    api::test_connection(api_type, &api_key, &base_url).await
}

/// Fetches available models from a provider.
#[tauri::command]
pub async fn fetch_models(
    provider_id: String,
    api_key: String,
    api_type: ApiType,
    base_url: Option<String>,
    state: tauri::State<'_, ConfigState>,
) -> Result<FetchModelsResult, String> {
    info!("fetch_models: provider_id={}, api_type={:?}, base_url={:?}", provider_id, api_type, base_url);

    // Empty API key check
    if api_key.trim().is_empty() {
        return Ok(FetchModelsResult::failure("Please enter an API key first"));
    }

    // Get provider to check models_source setting
    let store = state.store.lock().await;
    let provider = store
        .get_provider(&provider_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Provider {} not found", provider_id))?;

    info!("fetch_models: provider from DB - base_url={:?}, models_source={:?}", provider.base_url, provider.models_source);

    let models_source = provider.models_source;

    // Get manual models for this provider
    let manual_models = store
        .get_manual_models(&provider_id)
        .map_err(|e| e.to_string())?;

    // Release lock before making API call
    drop(store);

    // Fetch models from API (or use manual models depending on mode)
    let result = api::fetch_models_with_mode(
        api_type,
        &api_key,
        base_url.as_deref(),
        models_source,
        &manual_models,
    )
    .await?;

    if !result.success {
        return Ok(result);
    }

    // Store models in database
    let store = state.store.lock().await;
    let models: Vec<Model> = result
        .models
        .iter()
        .map(|name| {
            let caps = ModelCapabilities::infer_from_name(name);
            Model::new(&provider_id, name, caps)
        })
        .collect();

    store
        .save_models(&provider_id, &models)
        .map_err(|e| e.to_string())?;

    // Mark provider as configured
    store
        .set_provider_configured(&provider_id, true)
        .map_err(|e| e.to_string())?;

    // Return model names
    Ok(FetchModelsResult::success(
        models.iter().map(|m| m.name.clone()).collect(),
    ))
}

// =========================================================================
// API Key Commands
// =========================================================================

/// Stores an API key in the database.
#[tauri::command]
pub async fn store_api_key(
    provider_id: String,
    api_key: String,
    state: tauri::State<'_, ConfigState>,
) -> Result<(), String> {
    let store = state.store.lock().await;
    store
        .store_api_key(&provider_id, &api_key)
        .map_err(|e| e.to_string())
}

/// Retrieves an API key from the database.
#[tauri::command]
pub async fn retrieve_api_key(
    provider_id: String,
    state: tauri::State<'_, ConfigState>,
) -> Result<String, String> {
    let store = state.store.lock().await;
    store
        .retrieve_api_key(&provider_id)
        .map_err(|e| e.to_string())
}

/// Deletes an API key from the database.
#[tauri::command]
pub async fn delete_api_key(
    provider_id: String,
    state: tauri::State<'_, ConfigState>,
) -> Result<(), String> {
    let store = state.store.lock().await;
    store
        .delete_api_key(&provider_id)
        .map_err(|e| e.to_string())
}

/// Checks if an API key exists for a provider.
#[tauri::command]
pub async fn has_api_key(
    provider_id: String,
    state: tauri::State<'_, ConfigState>,
) -> Result<bool, String> {
    let store = state.store.lock().await;
    store.has_api_key(&provider_id).map_err(|e| e.to_string())
}

// =========================================================================
// Operation Default Commands
// =========================================================================

/// Gets all operation defaults.
#[tauri::command]
pub async fn get_operation_defaults(
    state: tauri::State<'_, ConfigState>,
) -> Result<Vec<OperationDefault>, String> {
    let store = state.store.lock().await;
    store
        .get_operation_defaults()
        .map_err(|e| e.to_string())
}

/// Sets the default model for an operation.
#[tauri::command]
pub async fn set_operation_default(
    operation: OperationType,
    model_id: String,
    state: tauri::State<'_, ConfigState>,
) -> Result<(), String> {
    let store = state.store.lock().await;

    // Verify model exists
    let model = store
        .get_model(&model_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Model {} not found", model_id))?;

    // Verify model supports the operation
    let supports = match operation {
        OperationType::Analysis => model.capabilities.analysis,
        OperationType::Code => model.capabilities.code,
        OperationType::Vision => model.capabilities.vision,
        OperationType::Audio => model.capabilities.audio,
        OperationType::Tips => model.capabilities.tips,
        OperationType::Review => model.capabilities.review,
    };

    if !supports {
        return Err(format!(
            "Model {} does not support operation {:?}",
            model.name, operation
        ));
    }

    store
        .set_operation_default(operation, &model_id)
        .map_err(|e| e.to_string())
}

// =========================================================================
// Model Commands
// =========================================================================

/// Gets all models for a provider.
#[tauri::command]
pub async fn get_models(
    provider_id: String,
    state: tauri::State<'_, ConfigState>,
) -> Result<Vec<Model>, String> {
    let store = state.store.lock().await;
    store.get_models(&provider_id).map_err(|e| e.to_string())
}

// =========================================================================
// Manual Model Commands
// =========================================================================

/// Gets all manual models for a provider.
#[tauri::command]
pub async fn get_manual_models(
    provider_id: String,
    state: tauri::State<'_, ConfigState>,
) -> Result<Vec<ManualModelEntry>, String> {
    let store = state.store.lock().await;
    store.get_manual_models(&provider_id).map_err(|e| e.to_string())
}

/// Adds a manual model for a provider.
#[tauri::command]
pub async fn add_manual_model(
    request: ManualModelRequest,
    state: tauri::State<'_, ConfigState>,
) -> Result<(), String> {
    let store = state.store.lock().await;

    // Validate model name
    let name = request.name.trim();
    if name.is_empty() {
        return Err("Model name cannot be empty".to_string());
    }

    // Use provided capabilities or infer from model name
    let capabilities = request
        .capabilities
        .unwrap_or_else(|| ModelCapabilities::infer_from_name(name));

    let model = ManualModelEntry::new(name.clone(), capabilities.clone());

    // Add to manual_models table
    store
        .add_manual_model(&request.provider_id, &model)
        .map_err(|e| e.to_string())?;

    // Also add to models table so it appears in SettingsModal
    let db_model = Model::new(&request.provider_id, &name, capabilities);
    store
        .add_model(&db_model)
        .map_err(|e| e.to_string())?;

    Ok(())
}

/// Removes a manual model for a provider.
#[tauri::command]
pub async fn remove_manual_model(
    provider_id: String,
    model_name: String,
    state: tauri::State<'_, ConfigState>,
) -> Result<(), String> {
    let store = state.store.lock().await;

    // Remove from manual_models table
    store
        .remove_manual_model(&provider_id, &model_name)
        .map_err(|e| e.to_string())?;

    // Also remove from models table
    store
        .delete_model_by_name(&provider_id, &model_name)
        .map_err(|e| e.to_string())?;

    Ok(())
}

/// Sets the models source mode for a provider.
#[tauri::command]
pub async fn set_provider_models_source(
    provider_id: String,
    models_source: ModelsSource,
    state: tauri::State<'_, ConfigState>,
) -> Result<(), String> {
    let store = state.store.lock().await;

    // Update the provider's models_source setting
    store
        .set_provider_models_source(&provider_id, models_source)
        .map_err(|e| e.to_string())?;

    // If switching to manual or hybrid mode, sync existing manual models to the models table
    if matches!(models_source, ModelsSource::Manual | ModelsSource::Hybrid) {
        let manual_models = store
            .get_manual_models(&provider_id)
            .map_err(|e| e.to_string())?;

        for manual_model in manual_models {
            let db_model = Model::new(&provider_id, &manual_model.name, manual_model.capabilities);
            store
                .add_model(&db_model)
                .map_err(|e| e.to_string())?;
        }
    }

    Ok(())
}

// =========================================================================
// LLM Chat Configuration Helpers
// =========================================================================

/// Gets the model ID configured for the 'code' operation.
///
/// This is a helper function for LLM chat commands to resolve which model
/// to use for code-related tasks.
///
/// # Errors
///
/// Returns an error if:
/// - No model is configured for the 'code' operation
/// - The configured model no longer exists in the database
pub fn get_code_operation_model(store: &ConfigStore) -> Result<Model, String> {
    let defaults = store
        .get_operation_defaults()
        .map_err(|e| format!("Failed to get operation defaults: {}", e))?;

    let code_default = defaults
        .into_iter()
        .find(|d| d.operation == OperationType::Code)
        .ok_or_else(|| {
            "No model configured for 'code' operation. Please configure a model first.".to_string()
        })?;

    let model_id = code_default
        .model_id
        .ok_or_else(|| "Code operation has a NULL model_id (database inconsistency)".to_string())?;

    store
        .get_model(&model_id)
        .map_err(|e| format!("Failed to get model {}: {}", model_id, e))?
        .ok_or_else(|| format!("Model {} no longer exists", model_id))
}

/// Response containing the complete LLM configuration for the 'code' operation.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CodeLlmConfigResponse {
    /// Model ID
    pub id: String,
    /// Model name (e.g., "gpt-4")
    pub name: String,
    /// Provider ID
    pub provider_id: String,
    /// Provider name
    pub provider_name: String,
    /// API type
    pub api_type: ApiType,
    /// Custom base URL (if any)
    pub base_url: Option<String>,
}

/// Gets the complete LLM configuration for the 'code' operation.
///
/// This includes model details and provider information needed to create
/// an LLM client. Does NOT include the API key - that must be retrieved
/// separately from the database.
///
/// # Errors
///
/// Returns an error if no model is configured for 'code' operation or
/// if the model/provider data is inconsistent.
pub fn get_code_llm_config(store: &ConfigStore) -> Result<CodeLlmConfigResponse, String> {
    let model = get_code_operation_model(store)?;

    let provider = store
        .get_provider(&model.provider_id)
        .map_err(|e| format!("Failed to get provider {}: {}", model.provider_id, e))?
        .ok_or_else(|| format!("Provider {} no longer exists", model.provider_id))?;

    Ok(CodeLlmConfigResponse {
        id: model.id,
        name: model.name,
        provider_id: provider.id,
        provider_name: provider.name,
        api_type: provider.api_type,
        base_url: provider.base_url,
    })
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_config_state_creation() {
        let state = ConfigState::in_memory().expect("Failed to create state");
        let store = state.store.lock().await;
        let providers = store.get_providers().expect("Failed to get providers");
        assert_eq!(providers.len(), 3); // Built-in providers
    }

    #[tokio::test]
    async fn test_store_operations() {
        let state = ConfigState::in_memory().expect("Failed to create state");
        let store = state.store.lock().await;

        // Test save provider
        let provider =
            Provider::custom("Test Provider", "TP", ApiType::OpenAI, None);
        store.save_provider(&provider).expect("Failed to save provider");

        // Test get providers
        let providers = store.get_providers().expect("Failed to get providers");
        assert!(providers.iter().any(|p| p.id == provider.id));

        // Test delete provider
        store
            .delete_provider(&provider.id)
            .expect("Failed to delete provider");
    }

    #[tokio::test]
    async fn test_builtin_providers_not_custom() {
        let state = ConfigState::in_memory().expect("Failed to create state");
        let store = state.store.lock().await;
        let providers = store.get_providers().expect("Failed to get providers");

        // Verify all built-in providers have is_custom = false
        for provider in providers {
            assert!(
                !provider.is_custom,
                "Built-in provider {} should have is_custom = false",
                provider.id
            );
        }
    }

    #[tokio::test]
    async fn test_cannot_delete_builtin_providers() {
        let state = ConfigState::in_memory().expect("Failed to create state");
        let store = state.store.lock().await;

        // Try to delete built-in providers - should fail
        for builtin_id in BUILTIN_PROVIDER_IDS {
            let result = store.delete_provider(builtin_id);
            assert!(
                result.is_err(),
                "Should not be able to delete built-in provider {}",
                builtin_id
            );
        }
    }

    // =========================================================================
    // Phase 1 Tests: Configuration Query
    // =========================================================================

    #[tokio::test]
    async fn test_get_code_operation_model_success() {
        let state = ConfigState::in_memory().expect("Failed to create state");
        let store = state.store.lock().await;

        // Setup: Create a model for OpenAI
        let model = Model::new("openai", "gpt-4", ModelCapabilities::default());
        store
            .save_models("openai", &[model.clone()])
            .expect("Failed to save model");

        // Set the code operation default
        store
            .set_operation_default(OperationType::Code, "openai-gpt-4")
            .expect("Failed to set operation default");

        // Test: Get the code operation model
        let result = get_code_operation_model(&store)
            .expect("Failed to get code operation model");

        assert_eq!(result.name, "gpt-4");
        assert_eq!(result.provider_id, "openai");
        assert_eq!(result.id, "openai-gpt-4");
    }

    #[tokio::test]
    async fn test_get_code_operation_model_not_configured() {
        let state = ConfigState::in_memory().expect("Failed to create state");
        let store = state.store.lock().await;

        // Test: No code operation configured
        let result = get_code_operation_model(&store);

        assert!(result.is_err());
        assert!(result.unwrap_err().contains("No model configured for 'code' operation"));
    }

    #[tokio::test]
    async fn test_get_code_llm_config_full() {
        let state = ConfigState::in_memory().expect("Failed to create state");
        let store = state.store.lock().await;

        // Setup: Create a model for OpenAI
        let model = Model::new("openai", "gpt-4-turbo", ModelCapabilities::default());
        store
            .save_models("openai", &[model.clone()])
            .expect("Failed to save model");

        // Set the code operation default
        store
            .set_operation_default(OperationType::Code, "openai-gpt-4-turbo")
            .expect("Failed to set operation default");

        // Test: Get full LLM config
        let result = get_code_llm_config(&store)
            .expect("Failed to get code LLM config");

        assert_eq!(result.name, "gpt-4-turbo");
        assert_eq!(result.provider_id, "openai");
        assert_eq!(result.provider_name, "OpenAI");
        assert_eq!(result.api_type, ApiType::OpenAI);
        assert_eq!(result.base_url, None);
    }
}

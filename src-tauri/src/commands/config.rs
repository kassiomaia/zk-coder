//! Configuration Tauri IPC commands.
//!
//! This module provides Tauri commands for LLM provider configuration management.

use std::sync::Arc;
use tokio::sync::Mutex;

use crate::api;
use crate::core::config::{
    ApiType, FetchModelsResult, ManualModelEntry, ManualModelRequest, Model, ModelCapabilities,
    ModelsSource, OperationDefault, OperationType, Provider, SaveProviderRequest, ValidationResult,
};
use crate::storage::ConfigStore;

// Re-export log macros for debugging
use log::{error, info};

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
        Provider::custom(
            &request.name,
            &request.icon,
            request.api_type,
            request.base_url,
        )
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
        return Ok(ValidationResult::failure("Please enter an API key first"));
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
    info!(
        "fetch_models: provider_id={}, api_type={:?}, base_url={:?}",
        provider_id, api_type, base_url
    );

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

    info!(
        "fetch_models: provider from DB - base_url={:?}, models_source={:?}",
        provider.base_url, provider.models_source
    );

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
    store.get_operation_defaults().map_err(|e| e.to_string())
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
    store
        .get_manual_models(&provider_id)
        .map_err(|e| e.to_string())
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
    store.add_model(&db_model).map_err(|e| e.to_string())?;

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
            store.add_model(&db_model).map_err(|e| e.to_string())?;
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

#[cfg(all(test, feature = "integration-test"))]
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
        let provider = Provider::custom("Test Provider", "TP", ApiType::OpenAI, None);
        store
            .save_provider(&provider)
            .expect("Failed to save provider");

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
        let result = get_code_operation_model(&store).expect("Failed to get code operation model");

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
        assert!(result
            .unwrap_err()
            .contains("No model configured for 'code' operation"));
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
        let result = get_code_llm_config(&store).expect("Failed to get code LLM config");

        assert_eq!(result.name, "gpt-4-turbo");
        assert_eq!(result.provider_id, "openai");
        assert_eq!(result.provider_name, "OpenAI");
        assert_eq!(result.api_type, ApiType::OpenAI);
        assert_eq!(result.base_url, None);
    }

    // =========================================================================
    // Command Tests (async)
    // =========================================================================

    #[tokio::test]
    async fn test_get_providers_command() {
        let state = ConfigState::in_memory().expect("Failed to create state");
        let providers = get_providers(tauri::State::from(&state))
            .await
            .expect("Failed to get providers");
        assert!(!providers.is_empty());
        assert_eq!(providers.len(), 3); // Built-in providers
    }

    #[tokio::test]
    async fn test_save_provider_new_custom() {
        let state = ConfigState::in_memory().expect("Failed to create state");
        let request = SaveProviderRequest {
            id: None,
            name: "Custom Provider".to_string(),
            icon: "CP".to_string(),
            api_type: ApiType::OpenAI,
            base_url: Some("https://custom.example.com".to_string()),
        };
        let result = save_provider(request, tauri::State::from(&state))
            .await
            .expect("Failed to save provider");
        assert_eq!(result.name, "Custom Provider");
        assert!(result.is_custom);
    }

    #[tokio::test]
    async fn test_save_provider_with_builtin_id_fails() {
        let state = ConfigState::in_memory().expect("Failed to create state");
        let request = SaveProviderRequest {
            id: Some("anthropic".to_string()),
            name: "Malicious".to_string(),
            icon: "M".to_string(),
            api_type: ApiType::Anthropic,
            base_url: None,
        };
        let result = save_provider(request, tauri::State::from(&state)).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("reserved ID"));
    }

    #[tokio::test]
    async fn test_save_provider_update_existing() {
        let state = ConfigState::in_memory().expect("Failed to create state");

        // First create a custom provider
        let create_request = SaveProviderRequest {
            id: None,
            name: "Test Provider".to_string(),
            icon: "TP".to_string(),
            api_type: ApiType::OpenAI,
            base_url: None,
        };
        let created = save_provider(create_request, tauri::State::from(&state))
            .await
            .expect("Failed to create provider");

        // Update it
        let update_request = SaveProviderRequest {
            id: Some(created.id.clone()),
            name: "Updated Provider".to_string(),
            icon: "UP".to_string(),
            api_type: ApiType::OpenAI,
            base_url: Some("https://updated.com".to_string()),
        };
        let result = save_provider(update_request, tauri::State::from(&state))
            .await
            .expect("Failed to update provider");
        assert_eq!(result.name, "Updated Provider");
        assert_eq!(result.base_url, Some("https://updated.com".to_string()));
    }

    #[tokio::test]
    async fn test_remove_provider_custom() {
        let state = ConfigState::in_memory().expect("Failed to create state");

        // Create a custom provider
        let request = SaveProviderRequest {
            id: None,
            name: "To Delete".to_string(),
            icon: "TD".to_string(),
            api_type: ApiType::OpenAI,
            base_url: None,
        };
        let created = save_provider(request, tauri::State::from(&state))
            .await
            .expect("Failed to create provider");

        // Delete it
        let result = remove_provider(created.id.clone(), tauri::State::from(&state))
            .await;
        assert!(result.is_ok());

        // Verify it's gone
        let providers = get_providers(tauri::State::from(&state))
            .await
            .expect("Failed to get providers");
        assert!(!providers.iter().any(|p| p.id == created.id));
    }

    #[tokio::test]
    async fn test_validate_api_key_empty() {
        let result = validate_api_key(
            "test".to_string(),
            "".to_string(),
            ApiType::OpenAI,
            None,
        )
        .await
        .expect("Failed to validate");
        assert!(!result.valid);
        assert!(result.message.contains("enter an API key"));
    }

    #[tokio::test]
    async fn test_validate_api_key_whitespace_only() {
        let result = validate_api_key(
            "test".to_string(),
            "   ".to_string(),
            ApiType::OpenAI,
            None,
        )
        .await
        .expect("Failed to validate");
        assert!(!result.valid);
    }

    #[tokio::test]
    async fn test_test_connection_empty_api_key() {
        let result = test_connection(
            "https://api.example.com".to_string(),
            "".to_string(),
            ApiType::OpenAI,
        )
        .await
        .expect("Failed to test connection");
        assert!(!result.valid);
    }

    #[tokio::test]
    async fn test_store_api_key() {
        let state = ConfigState::in_memory().expect("Failed to create state");
        let result = store_api_key(
            "test-provider".to_string(),
            "sk-test-key-123".to_string(),
            tauri::State::from(&state),
        )
        .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_retrieve_api_key() {
        let state = ConfigState::in_memory().expect("Failed to create state");
        let provider_id = "test-provider-retrieve";
        let api_key = "sk-test-key-456";

        // Store the key
        store_api_key(
            provider_id.to_string(),
            api_key.to_string(),
            tauri::State::from(&state),
        )
        .await
        .expect("Failed to store API key");

        // Retrieve it
        let result = retrieve_api_key(provider_id.to_string(), tauri::State::from(&state))
            .await
            .expect("Failed to retrieve API key");
        assert_eq!(result, api_key);
    }

    #[tokio::test]
    async fn test_has_api_key_true() {
        let state = ConfigState::in_memory().expect("Failed to create state");
        let provider_id = "test-provider-has";

        // Store a key
        store_api_key(
            provider_id.to_string(),
            "sk-test".to_string(),
            tauri::State::from(&state),
        )
        .await
        .expect("Failed to store API key");

        // Check if exists
        let result = has_api_key(provider_id.to_string(), tauri::State::from(&state))
            .await
            .expect("Failed to check API key");
        assert!(result);
    }

    #[tokio::test]
    async fn test_has_api_key_false() {
        let state = ConfigState::in_memory().expect("Failed to create state");
        let result = has_api_key("nonexistent-provider".to_string(), tauri::State::from(&state))
            .await
            .expect("Failed to check API key");
        assert!(!result);
    }

    #[tokio::test]
    async fn test_delete_api_key() {
        let state = ConfigState::in_memory().expect("Failed to create state");
        let provider_id = "test-provider-delete";

        // Store a key
        store_api_key(
            provider_id.to_string(),
            "sk-test".to_string(),
            tauri::State::from(&state),
        )
        .await
        .expect("Failed to store API key");

        // Delete it
        delete_api_key(provider_id.to_string(), tauri::State::from(&state))
            .await
            .expect("Failed to delete API key");

        // Verify it's gone
        let result = has_api_key(provider_id.to_string(), tauri::State::from(&state))
            .await
            .expect("Failed to check API key");
        assert!(!result);
    }

    #[tokio::test]
    async fn test_get_operation_defaults() {
        let state = ConfigState::in_memory().expect("Failed to create state");
        let result = get_operation_defaults(tauri::State::from(&state))
            .await
            .expect("Failed to get operation defaults");
        assert!(!result.is_empty());
    }

    #[tokio::test]
    async fn test_set_operation_default_success() {
        let state = ConfigState::in_memory().expect("Failed to create state");
        let store = state.store.lock().await;

        // Create a model first
        let model = Model::new("openai", "gpt-4", ModelCapabilities {
    analysis: true,
    code: true,
    vision: true,
    audio: true,
    tips: true,
    review: true,
});
        store
            .save_models("openai", &[model])
            .expect("Failed to save model");
        drop(store);

        // Set operation default
        let result = set_operation_default(
            OperationType::Code,
            "openai-gpt-4".to_string(),
            tauri::State::from(&state),
        )
        .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_set_operation_default_model_not_found() {
        let state = ConfigState::in_memory().expect("Failed to create state");
        let result = set_operation_default(
            OperationType::Code,
            "nonexistent-model".to_string(),
            tauri::State::from(&state),
        )
        .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_get_models() {
        let state = ConfigState::in_memory().expect("Failed to create state");
        let store = state.store.lock().await;

        // Create a model
        let model = Model::new("openai", "gpt-4", ModelCapabilities {
    analysis: true,
    code: true,
    vision: true,
    audio: true,
    tips: true,
    review: true,
});
        store
            .save_models("openai", &[model])
            .expect("Failed to save model");
        drop(store);

        // Get models
        let result = get_models("openai".to_string(), tauri::State::from(&state))
            .await
            .expect("Failed to get models");
        assert!(!result.is_empty());
        assert_eq!(result[0].name, "gpt-4");
    }

    #[tokio::test]
    async fn test_add_manual_model() {
        let state = ConfigState::in_memory().expect("Failed to create state");
        let store = state.store.lock().await;

        // Create a custom provider
        let provider = Provider::custom("Test", "T", ApiType::OpenAI, None);
        store
            .save_provider(&provider)
            .expect("Failed to save provider");
        drop(store);

        // Add manual model
        let request = ManualModelRequest {
            provider_id: provider.id.clone(),
            name: "custom-model".to_string(),
            capabilities: None,
        };
        let result = add_manual_model(request, tauri::State::from(&state))
            .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_add_manual_model_empty_name() {
        let state = ConfigState::in_memory().expect("Failed to create state");
        let request = ManualModelRequest {
            provider_id: "test".to_string(),
            name: "".to_string(),
            capabilities: None,
        };
        let result = add_manual_model(request, tauri::State::from(&state))
            .await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("cannot be empty"));
    }

    #[tokio::test]
    async fn test_remove_manual_model() {
        let state = ConfigState::in_memory().expect("Failed to create state");
        let store = state.store.lock().await;

        // Create a custom provider
        let provider = Provider::custom("Test", "T", ApiType::OpenAI, None);
        store
            .save_provider(&provider)
            .expect("Failed to save provider");
        drop(store);

        // Add manual model
        let request = ManualModelRequest {
            provider_id: provider.id.clone(),
            name: "custom-model".to_string(),
            capabilities: None,
        };
        add_manual_model(request, tauri::State::from(&state))
            .await
            .expect("Failed to add manual model");

        // Remove it
        let result = remove_manual_model(
            provider.id.clone(),
            "custom-model".to_string(),
            tauri::State::from(&state),
        )
        .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_get_manual_models() {
        let state = ConfigState::in_memory().expect("Failed to create state");
        let store = state.store.lock().await;

        // Create a custom provider
        let provider = Provider::custom("Test", "T", ApiType::OpenAI, None);
        store
            .save_provider(&provider)
            .expect("Failed to save provider");
        drop(store);

        // Add manual model
        let request = ManualModelRequest {
            provider_id: provider.id.clone(),
            name: "custom-model".to_string(),
            capabilities: None,
        };
        add_manual_model(request, tauri::State::from(&state))
            .await
            .expect("Failed to add manual model");

        // Get manual models
        let result = get_manual_models(provider.id.clone(), tauri::State::from(&state))
            .await
            .expect("Failed to get manual models");
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].name, "custom-model");
    }

    #[tokio::test]
    async fn test_set_provider_models_source() {
        let state = ConfigState::in_memory().expect("Failed to create state");
        let store = state.store.lock().await;

        // Create a custom provider
        let provider = Provider::custom("Test", "T", ApiType::OpenAI, None);
        store
            .save_provider(&provider)
            .expect("Failed to save provider");
        drop(store);

        // Set to manual
        let result = set_provider_models_source(
            provider.id.clone(),
            ModelsSource::Manual,
            tauri::State::from(&state),
        )
        .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_fetch_models_empty_api_key() {
        let state = ConfigState::in_memory().expect("Failed to create state");
        let result = fetch_models(
            "test".to_string(),
            "".to_string(),
            ApiType::OpenAI,
            None,
            tauri::State::from(&state),
        )
        .await
        .expect("Failed to fetch models");
        assert!(!result.success);
    }

    // =========================================================================
    // ModelCapabilities Tests
    // =========================================================================

    #[test]
    fn test_model_capabilities_default() {
        let caps = ModelCapabilities::default();
        assert!(!caps.analysis);
        assert!(!caps.code);
        assert!(!caps.vision);
        assert!(!caps.audio);
        assert!(!caps.tips);
        assert!(!caps.review);
    }

    #[test]
    fn test_model_capabilities_all() {
        let caps = ModelCapabilities {
    analysis: true,
    code: true,
    vision: true,
    audio: true,
    tips: true,
    review: true,
};
        assert!(caps.analysis);
        assert!(caps.code);
        assert!(caps.vision);
        assert!(caps.audio);
        assert!(caps.tips);
        assert!(caps.review);
    }

    #[test]
    fn test_model_capabilities_infer_from_name_gpt4() {
        let caps = ModelCapabilities::infer_from_name("gpt-4");
        assert!(caps.code);
        assert!(caps.analysis);
        assert!(caps.review);
    }

    #[test]
    fn test_model_capabilities_infer_from_name_vision() {
        let caps = ModelCapabilities::infer_from_name("gpt-4-vision");
        assert!(caps.vision);
        assert!(caps.code);
    }

    #[test]
    fn test_model_capabilities_infer_from_name_audio() {
        let caps = ModelCapabilities::infer_from_name("whisper-1");
        assert!(caps.audio);
    }

    // =========================================================================
    // Provider Tests
    // =========================================================================

    #[test]
    fn test_provider_custom() {
        let provider = Provider::custom("Test", "T", ApiType::OpenAI, None);
        assert!(provider.is_custom);
        assert_eq!(provider.api_type, ApiType::OpenAI);
        assert!(provider.base_url.is_none());
    }

    #[test]
    fn test_provider_custom_with_base_url() {
        let provider = Provider::custom(
            "Test",
            "T",
            ApiType::OpenAI,
            Some("https://custom.com".to_string()),
        );
        assert_eq!(provider.base_url, Some("https://custom.com".to_string()));
    }

    // =========================================================================
    // OperationType Tests
    // =========================================================================

    #[test]
    fn test_operation_type_display() {
        assert_eq!(OperationType::Analysis.to_string(), "analysis");
        assert_eq!(OperationType::Code.to_string(), "code");
        assert_eq!(OperationType::Vision.to_string(), "vision");
        assert_eq!(OperationType::Audio.to_string(), "audio");
        assert_eq!(OperationType::Tips.to_string(), "tips");
        assert_eq!(OperationType::Review.to_string(), "review");
    }

    // =========================================================================
    // Input Validation Tests
    // =========================================================================

    #[tokio::test]
    async fn test_provider_name_with_special_characters() {
        let state = ConfigState::in_memory().expect("Failed to create state");
        let request = SaveProviderRequest {
            id: None,
            name: "Test <Provider> & More!".to_string(),
            icon: "TP".to_string(),
            api_type: ApiType::OpenAI,
            base_url: None,
        };
        let result = save_provider(request, tauri::State::from(&state))
            .await
            .expect("Failed to save provider");
        assert_eq!(result.name, "Test <Provider> & More!");
    }

    #[tokio::test]
    async fn test_very_long_provider_name() {
        let state = ConfigState::in_memory().expect("Failed to create state");
        let long_name = "A".repeat(500);
        let request = SaveProviderRequest {
            id: None,
            name: long_name.clone(),
            icon: "TP".to_string(),
            api_type: ApiType::OpenAI,
            base_url: None,
        };
        let result = save_provider(request, tauri::State::from(&state))
            .await
            .expect("Failed to save provider");
        assert_eq!(result.name, long_name);
    }

    #[tokio::test]
    async fn test_invalid_base_url_format() {
        let state = ConfigState::in_memory().expect("Failed to create state");
        let request = SaveProviderRequest {
            id: None,
            name: "Test".to_string(),
            icon: "T".to_string(),
            api_type: ApiType::OpenAI,
            base_url: Some("not-a-url".to_string()),
        };
        // Command accepts any string; validation happens at API call time
        let result = save_provider(request, tauri::State::from(&state)).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_api_key_with_unicode() {
        let state = ConfigState::in_memory().expect("Failed to create state");
        let result = store_api_key(
            "test".to_string(),
            "sk-test-🔑-key".to_string(),
            tauri::State::from(&state),
        )
        .await;
        // Unicode in API keys should be accepted
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_model_id_with_dash() {
        let state = ConfigState::in_memory().expect("Failed to create state");
        let store = state.store.lock().await;

        let model = Model::new("test-provider", "model-with-dashes", ModelCapabilities {
    analysis: true,
    code: true,
    vision: true,
    audio: true,
    tips: true,
    review: true,
});
        store
            .save_models("test-provider", &[model])
            .expect("Failed to save model");
        drop(store);

        let result = get_models("test-provider".to_string(), tauri::State::from(&state))
            .await
            .expect("Failed to get models");
        assert_eq!(result[0].name, "model-with-dashes");
    }

    // =========================================================================
    // Error Handling Tests
    // =========================================================================

    #[tokio::test]
    async fn test_get_nonexistent_provider() {
        let state = ConfigState::in_memory().expect("Failed to create state");
        let store = state.store.lock().await;
        let result = store.get_provider("nonexistent");
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_delete_builtin_provider_fails() {
        let state = ConfigState::in_memory().expect("Failed to create state");
        let store = state.store.lock().await;
        let result = store.delete_provider("anthropic");
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_set_operation_default_unsupported_capability() {
        let state = ConfigState::in_memory().expect("Failed to create state");
        let store = state.store.lock().await;

        // Create a model without vision capability
        let mut caps = ModelCapabilities::default();
        caps.code = true;
        let model = Model::new("test", "model", caps);
        store
            .save_models("test", &[model])
            .expect("Failed to save model");
        drop(store);

        // Try to set as vision default
        let result = set_operation_default(
            OperationType::Vision,
            "test-model".to_string(),
            tauri::State::from(&state),
        )
        .await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("does not support"));
    }

    #[tokio::test]
    async fn test_models_source_variants() {
        // Test that all ModelsSource variants can be created
        let _manual = ModelsSource::Manual;
        let _api = ModelsSource::Api;
        let _hybrid = ModelsSource::Hybrid;
    }

    #[tokio::test]
    async fn test_api_type_variants() {
        // Test that all ApiType variants work
        let _anthropic = ApiType::Anthropic;
        let _openai = ApiType::OpenAI;
        let _google = ApiType::Google;
    }
}

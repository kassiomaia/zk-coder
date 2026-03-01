//! Configuration Tauri IPC commands.
//!
//! This module provides Tauri commands for LLM provider configuration management.

use std::sync::Arc;
use tokio::sync::Mutex;

use crate::api;
use crate::keychain;
use crate::nlp::config::{
    ApiType, FetchModelsResult, Model, ModelCapabilities, OperationDefault, OperationType,
    Provider, SaveProviderRequest, ValidationResult,
};
use crate::storage::ConfigStore;

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
    let store = state.store.lock().await;
    store.get_providers().map_err(|e| e.to_string())
}

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
            // Update existing provider
            Provider {
                id,
                name: request.name,
                icon: request.icon,
                is_custom: existing.is_custom,
                api_type: request.api_type,
                base_url: request.base_url,
                is_configured: existing.is_configured,
                created_at: existing.created_at,
                updated_at: chrono::Utc::now(),
            }
        } else {
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

    // Delete provider from database (cascade deletes models)
    store
        .delete_provider(&provider_id)
        .map_err(|e| e.to_string())?;

    // Clear operation defaults for this provider's models
    store
        .clear_defaults_for_provider(&provider_id)
        .map_err(|e| e.to_string())?;

    // Delete API key from keychain (ignore errors if not found)
    let _ = keychain::delete_api_key(&provider_id);

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
    // Empty API key check
    if api_key.trim().is_empty() {
        return Ok(FetchModelsResult::failure("Please enter an API key first"));
    }

    // Fetch models from API
    let result = api::fetch_models(api_type, &api_key, base_url.as_deref()).await?;

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
// Keychain Commands
// =========================================================================

/// Stores an API key in the OS keychain.
#[tauri::command]
pub async fn store_api_key(provider_id: String, api_key: String) -> Result<(), String> {
    keychain::store_api_key(&provider_id, &api_key).map_err(|e| e.to_string())
}

/// Retrieves an API key from the OS keychain.
#[tauri::command]
pub async fn retrieve_api_key(provider_id: String) -> Result<String, String> {
    keychain::retrieve_api_key(&provider_id).map_err(|e| e.to_string())
}

/// Deletes an API key from the OS keychain.
#[tauri::command]
pub async fn delete_api_key(provider_id: String) -> Result<(), String> {
    keychain::delete_api_key(&provider_id).map_err(|e| e.to_string())
}

/// Checks if an API key exists for a provider.
#[tauri::command]
pub async fn has_api_key(provider_id: String) -> Result<bool, String> {
    keychain::has_api_key(&provider_id).map_err(|e| e.to_string())
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
}

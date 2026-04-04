use log::info;

use crate::core::config::{ApiType, FetchModelsResult, Model, ModelCapabilities, ValidationResult};
use crate::infrastructure::api;

use super::state::ConfigState;

/// Validates an API key with the provider.
#[tauri::command]
pub async fn validate_api_key(
    _provider_id: String,
    api_key: String,
    api_type: ApiType,
    base_url: Option<String>,
) -> Result<ValidationResult, String> {
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
    info!("fetch_models: provider_id={provider_id}, api_type={api_type:?}, base_url={base_url:?}");

    if api_key.trim().is_empty() {
        return Ok(FetchModelsResult::failure("Please enter an API key first"));
    }

    let store = &state.repository;
    let provider = store
        .get_provider(&provider_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Provider {provider_id} not found"))?;

    info!(
        "fetch_models: provider from DB - base_url={:?}, models_source={:?}",
        provider.base_url, provider.models_source
    );

    let models_source = provider.models_source;
    let manual_models = store
        .get_manual_models(&provider_id)
        .map_err(|e| e.to_string())?;

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

    let store = &state.repository;
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
    store
        .set_provider_configured(&provider_id, true)
        .map_err(|e| e.to_string())?;

    Ok(FetchModelsResult::success(
        models.iter().map(|m| m.name.clone()).collect(),
    ))
}

/// Stores an API key in the database.
#[tauri::command]
pub async fn store_api_key(
    provider_id: String,
    api_key: String,
    state: tauri::State<'_, ConfigState>,
) -> Result<(), String> {
    state
        .repository
        .store_api_key(&provider_id, &api_key)
        .map_err(|e| e.to_string())
}

/// Retrieves an API key from the database.
#[tauri::command]
pub async fn retrieve_api_key(
    provider_id: String,
    state: tauri::State<'_, ConfigState>,
) -> Result<String, String> {
    state
        .repository
        .retrieve_api_key(&provider_id)
        .map_err(|e| e.to_string())
}

/// Deletes an API key from the database.
#[tauri::command]
pub async fn delete_api_key(
    provider_id: String,
    state: tauri::State<'_, ConfigState>,
) -> Result<(), String> {
    state
        .repository
        .delete_api_key(&provider_id)
        .map_err(|e| e.to_string())
}

/// Checks if an API key exists for a provider.
#[tauri::command]
pub async fn has_api_key(
    provider_id: String,
    state: tauri::State<'_, ConfigState>,
) -> Result<bool, String> {
    state
        .repository
        .has_api_key(&provider_id)
        .map_err(|e| e.to_string())
}

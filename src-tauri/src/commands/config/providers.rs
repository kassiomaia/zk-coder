use log::{error, info};

use crate::core::config::{Provider, SaveProviderRequest};

use super::helpers::BUILTIN_PROVIDER_IDS;
use super::state::ConfigState;

/// Gets all providers.
#[tauri::command]
pub async fn get_providers(state: tauri::State<'_, ConfigState>) -> Result<Vec<Provider>, String> {
    info!("get_providers: Starting to fetch providers...");
    let store = &state.repository;
    let providers = store.get_providers().map_err(|e| {
        error!("get_providers: Failed to get providers: {e}");
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

/// Saves a provider (create or update).
#[tauri::command]
pub async fn save_provider(
    request: SaveProviderRequest,
    state: tauri::State<'_, ConfigState>,
) -> Result<Provider, String> {
    let store = &state.repository;

    let provider = if let Some(id) = request.id {
        let existing = store.get_provider(&id).map_err(|e| e.to_string())?;

        if let Some(existing) = existing {
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
            if BUILTIN_PROVIDER_IDS.contains(&id.as_str()) {
                return Err(format!(
                    "Cannot create custom provider with reserved ID '{id}'"
                ));
            }
            Provider::custom_with_id(
                &id,
                &request.name,
                &request.icon,
                request.api_type,
                request.base_url,
            )
        }
    } else {
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
    let store = &state.repository;

    let _ = store.delete_api_key(&provider_id);

    store
        .clear_defaults_for_provider(&provider_id)
        .map_err(|e| e.to_string())?;

    store
        .delete_provider(&provider_id)
        .map_err(|e| e.to_string())?;

    Ok(())
}

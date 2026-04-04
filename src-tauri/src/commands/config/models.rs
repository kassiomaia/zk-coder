use crate::core::config::{
    ManualModelEntry, ManualModelRequest, Model, ModelCapabilities, ModelsSource,
};

use super::state::ConfigState;

/// Gets all models for a provider.
#[tauri::command]
pub async fn get_models(
    provider_id: String,
    state: tauri::State<'_, ConfigState>,
) -> Result<Vec<Model>, String> {
    state
        .repository
        .get_models(&provider_id)
        .map_err(|e| e.to_string())
}

/// Gets all manual models for a provider.
#[tauri::command]
pub async fn get_manual_models(
    provider_id: String,
    state: tauri::State<'_, ConfigState>,
) -> Result<Vec<ManualModelEntry>, String> {
    state
        .repository
        .get_manual_models(&provider_id)
        .map_err(|e| e.to_string())
}

/// Adds a manual model for a provider.
#[tauri::command]
pub async fn add_manual_model(
    request: ManualModelRequest,
    state: tauri::State<'_, ConfigState>,
) -> Result<(), String> {
    let store = &state.repository;
    let name = request.name.trim();
    if name.is_empty() {
        return Err("Model name cannot be empty".to_string());
    }

    let capabilities = request
        .capabilities
        .unwrap_or_else(|| ModelCapabilities::infer_from_name(name));

    let model = ManualModelEntry::new(name, capabilities.clone());

    store
        .add_manual_model(&request.provider_id, &model)
        .map_err(|e| e.to_string())?;

    let db_model = Model::new(&request.provider_id, name, capabilities);
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
    let store = &state.repository;

    store
        .remove_manual_model(&provider_id, &model_name)
        .map_err(|e| e.to_string())?;
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
    let store = &state.repository;

    store
        .set_provider_models_source(&provider_id, models_source)
        .map_err(|e| e.to_string())?;

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

use crate::core::config::{OperationDefault, OperationType};

use super::state::ConfigState;

/// Gets all operation defaults.
#[tauri::command]
pub async fn get_operation_defaults(
    state: tauri::State<'_, ConfigState>,
) -> Result<Vec<OperationDefault>, String> {
    state
        .repository
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
    let store = &state.repository;

    let model = store
        .get_model(&model_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Model {model_id} not found"))?;

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

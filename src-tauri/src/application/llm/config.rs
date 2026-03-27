use crate::core::config::{ApiType, ConfigRepository, Model, OperationType};

/// Response containing the complete LLM configuration for the `code` operation.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CodeLlmConfigResponse {
    pub id: String,
    pub name: String,
    pub provider_id: String,
    pub provider_name: String,
    pub api_type: ApiType,
    pub base_url: Option<String>,
}

/// Gets the model configured for the `code` operation.
pub fn get_code_operation_model(repository: &dyn ConfigRepository) -> Result<Model, String> {
    let defaults = repository
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

    repository
        .get_model(&model_id)
        .map_err(|e| format!("Failed to get model {}: {}", model_id, e))?
        .ok_or_else(|| format!("Model {} no longer exists", model_id))
}

/// Gets the complete provider/model configuration for the `code` operation.
pub fn get_code_llm_config(
    repository: &dyn ConfigRepository,
) -> Result<CodeLlmConfigResponse, String> {
    let model = get_code_operation_model(repository)?;

    let provider = repository
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

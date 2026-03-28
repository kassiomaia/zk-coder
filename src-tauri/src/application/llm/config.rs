//! Provides functions for retrieving LLM configuration for specific operations.

use crate::core::config::{ApiType, ConfigRepository, Model, OperationType};

/// A response containing the complete LLM configuration for the `code` operation.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CodeLlmConfigResponse {
    /// The unique identifier of the model.
    pub id: String,
    /// The name of the model.
    pub name: String,
    /// The unique identifier of the provider.
    pub provider_id: String,
    /// The name of the provider.
    pub provider_name: String,
    /// The type of the provider's API.
    pub api_type: ApiType,
    /// The custom base URL for the API, if applicable.
    pub base_url: Option<String>,
}

/// Retrieves the model that is configured as the default for the `code` operation.
///
/// # Arguments
///
/// * `repository` - A reference to the configuration repository.
///
/// # Errors
///
/// Returns an error if the operation default is not configured, or if the
/// configured model or provider no longer exists.
pub fn get_code_operation_model(repository: &dyn ConfigRepository) -> Result<Model, String> {
    let defaults = repository
        .get_operation_defaults()
        .map_err(|e| format!("Failed to get operation defaults: {e}"))?;

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
        .map_err(|e| format!("Failed to get model {model_id}: {e}"))?
        .ok_or_else(|| format!("Model {model_id} no longer exists"))
}

/// Retrieves the complete provider and model configuration for the `code` operation.
///
/// # Arguments
///
/// * `repository` - A reference to the configuration repository.
///
/// # Errors
///
/// Returns an error if the `code` operation model or its provider cannot be retrieved.
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

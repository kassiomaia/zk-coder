use log::info;

use crate::core::llm::entities::{
    ChatRequest as CoreChatRequest, ChatResponse as CoreChatResponse, LlmError, LlmResult,
};
use crate::core::llm::repositories::LlmProvider;
use crate::core::llm::services::LlmService;

use super::LlmServiceImpl;

fn block_on_runtime_safe<F, T>(future: F) -> Result<T, LlmError>
where
    F: std::future::Future<Output = T>,
{
    // `LlmService` is synchronous, but the provider clients are async. When this
    // is called from an async context (Tokio worker thread), creating a nested
    // runtime or calling `Runtime::block_on` will panic. `block_in_place` shifts
    // the blocking section off the worker thread.
    if let Ok(handle) = tokio::runtime::Handle::try_current() {
        Ok(tokio::task::block_in_place(|| handle.block_on(future)))
    } else {
        let rt = tokio::runtime::Runtime::new()
            .map_err(|e| LlmError::Unknown(format!("Failed to create runtime: {e}")))?;
        Ok(rt.block_on(future))
    }
}

impl LlmService for LlmServiceImpl {
    fn chat(&self, request: CoreChatRequest) -> LlmResult<CoreChatResponse> {
        let provider_id = infer_provider_from_model(&request.model);
        let client = self.create_client(provider_id)?;
        let llm_request = Self::convert_request(request);

        info!("Sending chat request to provider {provider_id}");

        let response = block_on_runtime_safe(client.chat(llm_request))?
            .map_err(|e| LlmError::Unknown(format!("Chat request failed: {e}")))?;

        Ok(Self::convert_to_core_response(response))
    }

    fn validate_api_key(&self, provider: LlmProvider, api_key: &str) -> LlmResult<bool> {
        let api_type = Self::convert_provider(provider);
        let client = Self::create_ephemeral_client(api_type, api_key);

        info!("Validating API key for provider {api_type:?}");

        let result = block_on_runtime_safe(client.validate_api_key())?
            .map_err(|e| LlmError::Unknown(format!("Validation failed: {e}")))?;

        info!("API key validation for provider {api_type:?} result: {result}");

        Ok(result)
    }

    fn fetch_models(&self, provider: LlmProvider, api_key: &str) -> LlmResult<Vec<String>> {
        let api_type = Self::convert_provider(provider);
        let client = Self::create_ephemeral_client(api_type, api_key);

        info!("Fetching models for provider {api_type:?}");

        let models = block_on_runtime_safe(client.fetch_models())?
            .map_err(|e| LlmError::Unknown(format!("Fetch models failed: {e}")))?;

        info!(
            "Fetched {} models for provider {:?}",
            models.len(),
            api_type
        );

        Ok(models)
    }

    fn get_default_model(&self, provider: LlmProvider) -> Option<String> {
        let api_type = Self::convert_provider(provider);
        Self::get_default_model_for_provider(api_type)
    }
}

/// Infers the provider ID from the model name.
pub fn infer_provider_from_model(model: &str) -> &'static str {
    let model_lower = model.to_lowercase();

    if model_lower.starts_with("claude") || model_lower.starts_with("glm") {
        "anthropic"
    } else if model_lower.starts_with("gemini") || model_lower.starts_with("google") {
        "google"
    } else {
        "openai"
    }
}

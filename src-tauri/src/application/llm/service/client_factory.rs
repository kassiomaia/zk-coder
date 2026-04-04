use log::debug;

use crate::core::config::{ApiType, Provider};
use crate::core::llm::entities::{LlmError, LlmResult};
use crate::infrastructure::llm::{anthropic::AnthropicClient, openai::OpenAiClient, LlmClient};

use super::LlmServiceImpl;

impl LlmServiceImpl {
    /// Gets a provider by ID from the database.
    pub(crate) fn get_provider_config(&self, provider_id: &str) -> LlmResult<Provider> {
        self.config_repository
            .get_provider(provider_id)
            .map_err(|e| LlmError::Unknown(format!("Failed to get provider: {e}")))?
            .ok_or_else(|| LlmError::Unknown(format!("Provider not found: {provider_id}")))
    }

    /// Creates an LLM client for the given provider ID.
    pub(crate) fn create_client(&self, provider_id: &str) -> LlmResult<Box<dyn LlmClient>> {
        let provider = self.get_provider_config(provider_id)?;
        let api_key = self
            .config_repository
            .retrieve_api_key(provider_id)
            .map_err(|e| LlmError::Unknown(format!("Failed to get API key: {e}")))?;

        let client: Box<dyn LlmClient> = match provider.api_type {
            ApiType::OpenAI => Box::new(OpenAiClient::new(api_key, provider.base_url)),
            ApiType::Google => Box::new(OpenAiClient::new_google(api_key)),
            ApiType::Anthropic => Box::new(AnthropicClient::new(api_key, provider.base_url)),
        };

        debug!(
            "Created LLM client for provider {} (type: {:?})",
            provider_id, provider.api_type
        );

        Ok(client)
    }

    pub(crate) fn create_ephemeral_client(api_type: ApiType, api_key: &str) -> Box<dyn LlmClient> {
        match api_type {
            ApiType::OpenAI => Box::new(OpenAiClient::new(api_key.to_string(), None)),
            ApiType::Google => Box::new(OpenAiClient::new_google(api_key.to_string())),
            ApiType::Anthropic => Box::new(AnthropicClient::new(api_key.to_string(), None)),
        }
    }
}

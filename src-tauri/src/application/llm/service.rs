//! LLM service implementation.
//!
//! This module provides the concrete implementation of the LlmService trait
//! defined in the core layer. It coordinates between the domain entities and
//! the infrastructure layer (OpenAI, Anthropic clients).

use log::{debug, info};
use std::sync::Arc;

use crate::core::llm::entities::{
    ChatMessage, ChatRequest as CoreChatRequest, ChatResponse as CoreChatResponse,
    ContentBlock as CoreContentBlock, ImageSource as CoreImageSource, LlmError, LlmResult,
    MessageContent, MessageRole as CoreMessageRole, ToolCall, UsageStats as CoreUsageStats,
};
use crate::core::llm::repositories::LlmProvider;
use crate::core::llm::services::LlmService;
use crate::llm::{
    anthropic::AnthropicClient, openai::OpenAiClient, ChatRequest, ChatResponse, ContentBlock,
    ImageSource, LlmClient, Message, MessageContent as LlmMessageContent, MessageRole as LlmMessageRole,
};
use crate::nlp::config::{ApiType, Provider};
use crate::storage::ConfigStore;

/// Implementation of the LLM service.
///
/// This service manages provider clients and provides high-level LLM operations.
pub struct LlmServiceImpl {
    /// Config store for provider and API key access
    config_store: Arc<ConfigStore>,
}

impl LlmServiceImpl {
    /// Creates a new LLM service instance.
    #[must_use]
    pub fn new(config_store: Arc<ConfigStore>) -> Self {
        Self { config_store }
    }

    /// Gets a provider by ID from the database.
    ///
    /// # Errors
    /// Returns an error if the provider cannot be found.
    fn get_provider_config(&self, provider_id: &str) -> LlmResult<Provider> {
        self.config_store
            .get_provider(provider_id)
            .map_err(|e| LlmError::Unknown(format!("Failed to get provider: {}", e)))?
            .ok_or_else(|| LlmError::Unknown(format!("Provider not found: {}", provider_id)))
    }

    /// Creates an LLM client for the given provider ID.
    ///
    /// # Errors
    /// Returns an error if the provider doesn't exist or API key retrieval fails.
    fn create_client(&self, provider_id: &str) -> LlmResult<Box<dyn LlmClient>> {
        let provider = self.get_provider_config(provider_id)?;
        let api_key = self
            .config_store
            .retrieve_api_key(provider_id)
            .map_err(|e| LlmError::Unknown(format!("Failed to get API key: {}", e)))?;

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

    /// Converts core message role to LLM module message role.
    fn convert_message_role(role: CoreMessageRole) -> LlmMessageRole {
        match role {
            CoreMessageRole::System => LlmMessageRole::System,
            CoreMessageRole::User => LlmMessageRole::User,
            CoreMessageRole::Assistant => LlmMessageRole::Assistant,
            CoreMessageRole::Tool => LlmMessageRole::Tool,
        }
    }

    /// Converts LLM module message role to core message role.
    fn convert_to_core_role(role: LlmMessageRole) -> CoreMessageRole {
        match role {
            LlmMessageRole::System => CoreMessageRole::System,
            LlmMessageRole::User => CoreMessageRole::User,
            LlmMessageRole::Assistant => CoreMessageRole::Assistant,
            LlmMessageRole::Tool => CoreMessageRole::Tool,
        }
    }

    /// Converts core ContentBlock to LLM module ContentBlock.
    fn convert_content_block(block: CoreContentBlock) -> ContentBlock {
        match block {
            CoreContentBlock::Text { text } => ContentBlock::Text { text },
            CoreContentBlock::Image { source } => ContentBlock::Image {
                source: ImageSource {
                    source_type: source.source_type,
                    media_type: source.media_type,
                    data: source.data,
                },
            },
        }
    }

    /// Converts LLM module ContentBlock to core ContentBlock.
    fn convert_to_core_content_block(block: ContentBlock) -> CoreContentBlock {
        match block {
            ContentBlock::Text { text } => CoreContentBlock::Text { text },
            ContentBlock::Image { source } => CoreContentBlock::Image {
                source: CoreImageSource {
                    source_type: source.source_type,
                    media_type: source.media_type,
                    data: source.data,
                },
            },
        }
    }

    /// Converts core message to LLM module message.
    fn convert_message(msg: ChatMessage) -> Message {
        Message {
            role: Self::convert_message_role(msg.role),
            content: match msg.content {
                MessageContent::Text(text) => LlmMessageContent::Text(text),
                MessageContent::Blocks(blocks) => {
                    LlmMessageContent::Blocks(blocks.into_iter().map(Self::convert_content_block).collect())
                }
            },
            tool_calls: msg
                .tool_calls
                .into_iter()
                .map(|tc| crate::llm::ToolCall {
                    id: tc.id,
                    name: tc.name,
                    arguments: tc.arguments,
                })
                .collect(),
            tool_result: msg.tool_result.map(|tr| crate::llm::ToolResult {
                call_id: tr.call_id,
                output: tr.output,
                is_error: tr.is_error,
            }),
        }
    }

    /// Converts LLM module message to core message.
    fn convert_to_core_message(msg: Message) -> ChatMessage {
        ChatMessage {
            role: Self::convert_to_core_role(msg.role),
            content: match msg.content {
                LlmMessageContent::Text(text) => MessageContent::Text(text),
                LlmMessageContent::Blocks(blocks) => {
                    MessageContent::Blocks(blocks.into_iter().map(Self::convert_to_core_content_block).collect())
                }
            },
            tool_calls: msg
                .tool_calls
                .into_iter()
                .map(|tc| ToolCall {
                    id: tc.id,
                    name: tc.name,
                    arguments: tc.arguments,
                })
                .collect(),
            tool_result: msg.tool_result.map(|tr| crate::core::llm::entities::ToolResult {
                call_id: tr.call_id,
                output: tr.output,
                is_error: tr.is_error,
            }),
        }
    }

    /// Converts core chat request to LLM module request.
    fn convert_request(req: CoreChatRequest) -> ChatRequest {
        ChatRequest {
            model: req.model,
            messages: req
                .messages
                .into_iter()
                .map(Self::convert_message)
                .collect(),
            system: req.system,
            temperature: req.temperature,
            max_tokens: req.max_tokens,
            top_p: None, // Core ChatRequest doesn't have this field
            stop: None,  // Core ChatRequest doesn't have this field
            stream: req.stream,
            tools: req
                .tools
                .into_iter()
                .map(|t| crate::llm::ToolDefinition {
                    name: t.name,
                    description: t.description,
                    input_schema: t.input_schema,
                })
                .collect(),
        }
    }

    /// Converts LLM module response to core response.
    fn convert_to_core_response(resp: ChatResponse) -> CoreChatResponse {
        CoreChatResponse {
            id: resp.id,
            model: resp.model,
            content: resp.content,
            usage: resp.usage.map(|u| CoreUsageStats {
                input_tokens: u.input_tokens,
                output_tokens: u.output_tokens,
                total_tokens: u.total_tokens,
            }),
            finish_reason: resp.finish_reason,
            tool_calls: resp
                .tool_calls
                .into_iter()
                .map(|tc| ToolCall {
                    id: tc.id,
                    name: tc.name,
                    arguments: tc.arguments,
                })
                .collect(),
        }
    }

    /// Converts core LLM provider to repository LLM provider.
    fn convert_provider(provider: LlmProvider) -> ApiType {
        match provider {
            LlmProvider::OpenAI => ApiType::OpenAI,
            LlmProvider::Anthropic => ApiType::Anthropic,
            LlmProvider::Google => ApiType::Google,
            LlmProvider::Custom { .. } => ApiType::OpenAI,
        }
    }

    /// Gets the default model for a provider.
    fn get_default_model_for_provider(api_type: ApiType) -> Option<String> {
        match api_type {
            ApiType::OpenAI => Some("gpt-4o".to_string()),
            ApiType::Anthropic => Some("claude-3-5-sonnet-20241022".to_string()),
            ApiType::Google => Some("gemini-2.0-flash-exp".to_string()),
        }
    }
}

impl LlmService for LlmServiceImpl {
    fn chat(&self, request: CoreChatRequest) -> LlmResult<CoreChatResponse> {
        // Infer provider from model name
        let provider_id = infer_provider_from_model(&request.model);
        let client = self.create_client(provider_id)?;

        let llm_request = Self::convert_request(request);

        info!("Sending chat request to provider {}", provider_id);

        // Use a blocking runtime for the async client
        let rt = tokio::runtime::Runtime::new()
            .map_err(|e| LlmError::Unknown(format!("Failed to create runtime: {}", e)))?;

        let response = rt
            .block_on(client.chat(llm_request))
            .map_err(|e| LlmError::Unknown(format!("Chat request failed: {}", e)))?;

        Ok(Self::convert_to_core_response(response))
    }

    fn validate_api_key(&self, provider: LlmProvider, api_key: &str) -> LlmResult<bool> {
        let api_type = Self::convert_provider(provider);

        // Create a temporary client for validation
        let client: Box<dyn LlmClient> = match api_type {
            ApiType::OpenAI => Box::new(OpenAiClient::new(api_key.to_string(), None)),
            ApiType::Google => Box::new(OpenAiClient::new_google(api_key.to_string())),
            ApiType::Anthropic => Box::new(AnthropicClient::new(api_key.to_string(), None)),
        };

        info!("Validating API key for provider {:?}", api_type);

        let rt = tokio::runtime::Runtime::new()
            .map_err(|e| LlmError::Unknown(format!("Failed to create runtime: {}", e)))?;

        let result = rt
            .block_on(client.validate_api_key())
            .map_err(|e| LlmError::Unknown(format!("Validation failed: {}", e)))?;

        info!(
            "API key validation for provider {:?} result: {}",
            api_type, result
        );

        Ok(result)
    }

    fn fetch_models(&self, provider: LlmProvider, api_key: &str) -> LlmResult<Vec<String>> {
        let api_type = Self::convert_provider(provider);

        // Create a temporary client for fetching models
        let client: Box<dyn LlmClient> = match api_type {
            ApiType::OpenAI => Box::new(OpenAiClient::new(api_key.to_string(), None)),
            ApiType::Google => Box::new(OpenAiClient::new_google(api_key.to_string())),
            ApiType::Anthropic => Box::new(AnthropicClient::new(api_key.to_string(), None)),
        };

        info!("Fetching models for provider {:?}", api_type);

        let rt = tokio::runtime::Runtime::new()
            .map_err(|e| LlmError::Unknown(format!("Failed to create runtime: {}", e)))?;

        let models = rt
            .block_on(client.fetch_models())
            .map_err(|e| LlmError::Unknown(format!("Fetch models failed: {}", e)))?;

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
///
/// This is a simple heuristic that can be improved with proper configuration.
fn infer_provider_from_model(model: &str) -> &'static str {
    let model_lower = model.to_lowercase();

    if model_lower.starts_with("claude") || model_lower.starts_with("glm") {
        "anthropic"
    } else if model_lower.starts_with("gemini") || model_lower.starts_with("google") {
        "google"
    } else {
        "openai"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_test_service() -> LlmServiceImpl {
        let config_store = ConfigStore::in_memory().expect("Failed to create config store");
        LlmServiceImpl::new(Arc::new(config_store))
    }

    #[test]
    fn test_service_creation() {
        let service = make_test_service();
        // Service should be created successfully
        assert_eq!(
            service.get_default_model(LlmProvider::OpenAI).unwrap(),
            "gpt-4o"
        );
    }

    #[test]
    fn test_get_default_model() {
        let service = make_test_service();

        assert_eq!(
            service.get_default_model(LlmProvider::OpenAI).unwrap(),
            "gpt-4o"
        );
        assert_eq!(
            service.get_default_model(LlmProvider::Anthropic).unwrap(),
            "claude-3-5-sonnet-20241022"
        );
        assert_eq!(
            service.get_default_model(LlmProvider::Google).unwrap(),
            "gemini-2.0-flash-exp"
        );
    }

    #[test]
    fn test_infer_provider_from_model() {
        assert_eq!(infer_provider_from_model("gpt-4o"), "openai");
        assert_eq!(infer_provider_from_model("claude-3-opus"), "anthropic");
        assert_eq!(infer_provider_from_model("gemini-pro"), "google");
        assert_eq!(infer_provider_from_model("GLM-4"), "anthropic"); // z.ai uses Anthropic-compatible API
    }

    #[test]
    fn test_convert_message_role() {
        assert_eq!(
            LlmServiceImpl::convert_message_role(CoreMessageRole::System),
            LlmMessageRole::System
        );
        assert_eq!(
            LlmServiceImpl::convert_message_role(CoreMessageRole::User),
            LlmMessageRole::User
        );
        assert_eq!(
            LlmServiceImpl::convert_message_role(CoreMessageRole::Assistant),
            LlmMessageRole::Assistant
        );
        assert_eq!(
            LlmServiceImpl::convert_message_role(CoreMessageRole::Tool),
            LlmMessageRole::Tool
        );
    }

    #[test]
    fn test_convert_provider() {
        assert_eq!(
            LlmServiceImpl::convert_provider(LlmProvider::OpenAI),
            ApiType::OpenAI
        );
        assert_eq!(
            LlmServiceImpl::convert_provider(LlmProvider::Anthropic),
            ApiType::Anthropic
        );
        assert_eq!(
            LlmServiceImpl::convert_provider(LlmProvider::Google),
            ApiType::Google
        );
    }
}

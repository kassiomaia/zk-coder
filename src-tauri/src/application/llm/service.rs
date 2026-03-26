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
use crate::core::config::{ApiType, Provider};
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
    pub(crate) fn get_provider_config(&self, provider_id: &str) -> LlmResult<Provider> {
        self.config_store
            .get_provider(provider_id)
            .map_err(|e| LlmError::Unknown(format!("Failed to get provider: {}", e)))?
            .ok_or_else(|| LlmError::Unknown(format!("Provider not found: {}", provider_id)))
    }

    /// Creates an LLM client for the given provider ID.
    ///
    /// # Errors
    /// Returns an error if the provider doesn't exist or API key retrieval fails.
    pub(crate) fn create_client(&self, provider_id: &str) -> LlmResult<Box<dyn LlmClient>> {
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
    pub(crate) fn convert_message_role(role: CoreMessageRole) -> LlmMessageRole {
        match role {
            CoreMessageRole::System => LlmMessageRole::System,
            CoreMessageRole::User => LlmMessageRole::User,
            CoreMessageRole::Assistant => LlmMessageRole::Assistant,
            CoreMessageRole::Tool => LlmMessageRole::Tool,
        }
    }

    /// Converts LLM module message role to core message role.
    pub(crate) fn convert_to_core_role(role: LlmMessageRole) -> CoreMessageRole {
        match role {
            LlmMessageRole::System => CoreMessageRole::System,
            LlmMessageRole::User => CoreMessageRole::User,
            LlmMessageRole::Assistant => CoreMessageRole::Assistant,
            LlmMessageRole::Tool => CoreMessageRole::Tool,
        }
    }

    /// Converts core ContentBlock to LLM module ContentBlock.
    pub(crate) fn convert_content_block(block: CoreContentBlock) -> ContentBlock {
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
    pub(crate) fn convert_to_core_content_block(block: ContentBlock) -> CoreContentBlock {
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
    pub(crate) fn convert_message(msg: ChatMessage) -> Message {
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
    pub(crate) fn convert_to_core_message(msg: Message) -> ChatMessage {
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
    pub(crate) fn convert_request(req: CoreChatRequest) -> ChatRequest {
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
    pub(crate) fn convert_to_core_response(resp: ChatResponse) -> CoreChatResponse {
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
    pub(crate) fn convert_provider(provider: LlmProvider) -> ApiType {
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
    use crate::core::llm::entities::{
        ChatMessage, ChatRequest, ContentBlock, ImageSource, MessageContent,
        MessageRole as CoreMessageRole, ToolCall, UsageStats,
    };
    use crate::core::config::{ApiType, Provider};

    fn make_test_service() -> LlmServiceImpl {
        let config_store = ConfigStore::in_memory().expect("Failed to create config store");
        LlmServiceImpl::new(Arc::new(config_store))
    }

    // ========================================================================
    // Service Creation Tests
    // ========================================================================

    #[test]
    fn test_service_creation() {
        let service = make_test_service();
        // Service should be created successfully
        assert_eq!(
            service.get_default_model(LlmProvider::OpenAI).unwrap(),
            "gpt-4o"
        );
    }

    // ========================================================================
    // Default Model Tests
    // ========================================================================

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
    fn test_get_default_model_for_custom_provider() {
        let service = make_test_service();

        // Custom providers should use OpenAI-compatible defaults
        let result = service.get_default_model(LlmProvider::Custom {
            name: "custom-provider".to_string(),
        });
        assert_eq!(result.unwrap(), "gpt-4o");
    }

    // ========================================================================
    // Provider Inference Tests
    // ========================================================================

    #[test]
    fn test_infer_provider_from_model() {
        assert_eq!(infer_provider_from_model("gpt-4o"), "openai");
        assert_eq!(infer_provider_from_model("gpt-3.5-turbo"), "openai");
        assert_eq!(infer_provider_from_model("claude-3-opus"), "anthropic");
        assert_eq!(infer_provider_from_model("claude-3-5-sonnet"), "anthropic");
        assert_eq!(infer_provider_from_model("gemini-pro"), "google");
        assert_eq!(infer_provider_from_model("gemini-2.0-flash"), "google");
        assert_eq!(infer_provider_from_model("GLM-4"), "anthropic"); // z.ai uses Anthropic-compatible API
    }

    #[test]
    fn test_infer_provider_case_insensitive() {
        assert_eq!(infer_provider_from_model("GPT-4"), "openai");
        assert_eq!(infer_provider_from_model("Claude-3"), "anthropic");
        assert_eq!(infer_provider_from_model("GEMINI-PRO"), "google");
    }

    // ========================================================================
    // Message Role Conversion Tests
    // ========================================================================

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
    fn test_convert_to_core_role_roundtrip() {
        let core_roles = vec![
            CoreMessageRole::System,
            CoreMessageRole::User,
            CoreMessageRole::Assistant,
            CoreMessageRole::Tool,
        ];

        for role in core_roles {
            let llm_role = LlmServiceImpl::convert_message_role(role);
            let back = LlmServiceImpl::convert_to_core_role(llm_role);
            assert_eq!(role, back);
        }
    }

    // ========================================================================
    // Provider Conversion Tests
    // ========================================================================

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

    #[test]
    fn test_convert_custom_provider() {
        // Custom providers map to OpenAI API type
        let result = LlmServiceImpl::convert_provider(LlmProvider::Custom {
            name: "custom-openai-compatible".to_string(),
        });
        assert_eq!(result, ApiType::OpenAI);
    }

    // ========================================================================
    // Message Conversion Tests
    // ========================================================================

    #[test]
    fn test_convert_text_message() {
        let core_msg = ChatMessage {
            role: CoreMessageRole::User,
            content: MessageContent::Text("Hello world".to_string()),
            tool_calls: vec![],
            tool_result: None,
        };

        let llm_msg = LlmServiceImpl::convert_message(core_msg);

        assert_eq!(llm_msg.role, LlmMessageRole::User);
        assert!(matches!(llm_msg.content, crate::llm::MessageContent::Text(_)));
    }

    #[test]
    fn test_convert_message_with_blocks() {
        let core_msg = ChatMessage {
            role: CoreMessageRole::User,
            content: MessageContent::Blocks(vec![
                ContentBlock::Text {
                    text: "Hello".to_string(),
                },
                ContentBlock::Image {
                    source: ImageSource {
                        source_type: "base64".to_string(),
                        media_type: Some("image/png".to_string()),
                        data: "iVBORw0KG...".to_string(),
                    },
                },
            ]),
            tool_calls: vec![],
            tool_result: None,
        };

        let llm_msg = LlmServiceImpl::convert_message(core_msg);

        assert!(matches!(llm_msg.content, crate::llm::MessageContent::Blocks(_)));
    }

    #[test]
    fn test_convert_message_with_tool_calls() {
        let core_msg = ChatMessage {
            role: CoreMessageRole::Assistant,
            content: MessageContent::Text("I'll help with that".to_string()),
            tool_calls: vec![ToolCall {
                id: "call-123".to_string(),
                name: "fs_read_file".to_string(),
                arguments: serde_json::json!({"path": "/test"}),
            }],
            tool_result: None,
        };

        let llm_msg = LlmServiceImpl::convert_message(core_msg);

        assert_eq!(llm_msg.tool_calls.len(), 1);
        assert_eq!(llm_msg.tool_calls[0].id, "call-123");
        assert_eq!(llm_msg.tool_calls[0].name, "fs_read_file");
    }

    #[test]
    fn test_convert_message_with_tool_result() {
        let core_msg = ChatMessage {
            role: CoreMessageRole::Tool,
            content: MessageContent::Text("Result".to_string()),
            tool_calls: vec![],
            tool_result: Some(crate::core::llm::entities::ToolResult {
                call_id: "call-123".to_string(),
                output: serde_json::json!({"result": "success"}),
                is_error: false,
            }),
        };

        let llm_msg = LlmServiceImpl::convert_message(core_msg);

        assert!(llm_msg.tool_result.is_some());
        assert_eq!(llm_msg.tool_result.as_ref().unwrap().call_id, "call-123");
        assert!(!llm_msg.tool_result.as_ref().unwrap().is_error);
    }

    // ========================================================================
    // Request Conversion Tests
    // ========================================================================

    #[test]
    fn test_convert_request_basic() {
        let core_req = ChatRequest {
            model: "gpt-4".to_string(),
            messages: vec![
                ChatMessage::user("Hello"),
                ChatMessage::assistant("Hi there"),
            ],
            system: None,
            temperature: Some(0.7),
            max_tokens: Some(1000),
            stream: false,
            tools: vec![],
        };

        let llm_req = LlmServiceImpl::convert_request(core_req);

        assert_eq!(llm_req.model, "gpt-4");
        assert_eq!(llm_req.messages.len(), 2);
        assert_eq!(llm_req.temperature, Some(0.7));
        assert_eq!(llm_req.max_tokens, Some(1000));
        assert_eq!(llm_req.stream, false);
    }

    #[test]
    fn test_convert_request_with_system() {
        let core_req = ChatRequest {
            model: "claude-3".to_string(),
            messages: vec![ChatMessage::user("Test")],
            system: Some("You are a helpful assistant".to_string()),
            temperature: None,
            max_tokens: None,
            stream: false,
            tools: vec![],
        };

        let llm_req = LlmServiceImpl::convert_request(core_req);

        assert_eq!(llm_req.system, Some("You are a helpful assistant".to_string()));
    }

    #[test]
    fn test_convert_request_with_tools() {
        let core_req = ChatRequest {
            model: "gpt-4".to_string(),
            messages: vec![ChatMessage::user("Use tools")],
            system: None,
            temperature: None,
            max_tokens: None,
            stream: false,
            tools: vec![crate::core::llm::entities::ToolDefinition {
                name: "fs_read_file".to_string(),
                description: "Read a file".to_string(),
                input_schema: serde_json::json!({"type": "object"}),
            }],
        };

        let llm_req = LlmServiceImpl::convert_request(core_req);

        assert_eq!(llm_req.tools.len(), 1);
        assert_eq!(llm_req.tools[0].name, "fs_read_file");
    }

    // ========================================================================
    // Content Block Conversion Tests
    // ========================================================================

    #[test]
    fn test_convert_content_block_text() {
        let core_block = ContentBlock::Text {
            text: "Hello world".to_string(),
        };

        let llm_block = LlmServiceImpl::convert_content_block(core_block);

        assert!(matches!(llm_block, crate::llm::ContentBlock::Text { .. }));
    }

    #[test]
    fn test_convert_content_block_image() {
        let core_block = ContentBlock::Image {
            source: ImageSource {
                source_type: "base64".to_string(),
                media_type: Some("image/png".to_string()),
                data: "base64data".to_string(),
            },
        };

        let llm_block = LlmServiceImpl::convert_content_block(core_block);

        assert!(matches!(llm_block, crate::llm::ContentBlock::Image { .. }));
        if let crate::llm::ContentBlock::Image { source } = llm_block {
            assert_eq!(source.source_type, "base64");
            assert_eq!(source.media_type, Some("image/png".to_string()));
            assert_eq!(source.data, "base64data");
        }
    }

    #[test]
    fn test_convert_to_core_content_block_roundtrip() {
        let core_block = ContentBlock::Text {
            text: "Test".to_string(),
        };

        let llm_block = LlmServiceImpl::convert_content_block(core_block.clone());
        let back = LlmServiceImpl::convert_to_core_content_block(llm_block);

        assert!(matches!(back, ContentBlock::Text { .. }));
        if let ContentBlock::Text { text } = back {
            assert_eq!(text, "Test");
        }
    }

    // ========================================================================
    // Response Conversion Tests
    // ========================================================================

    #[test]
    fn test_convert_to_core_response() {
        let llm_resp = crate::llm::ChatResponse {
            id: "resp-123".to_string(),
            model: "gpt-4".to_string(),
            content: "Hello there!".to_string(),
            usage: Some(crate::llm::UsageStats {
                input_tokens: 10,
                output_tokens: 20,
                total_tokens: 30,
            }),
            finish_reason: Some("stop".to_string()),
            tool_calls: vec![],
        };

        let core_resp = LlmServiceImpl::convert_to_core_response(llm_resp);

        assert_eq!(core_resp.id, "resp-123");
        assert_eq!(core_resp.model, "gpt-4");
        assert_eq!(core_resp.content, "Hello there!");
        assert_eq!(core_resp.finish_reason, Some("stop".to_string()));
        assert!(core_resp.usage.is_some());
        assert_eq!(core_resp.usage.as_ref().unwrap().total_tokens, 30);
    }

    #[test]
    fn test_convert_to_core_response_with_tool_calls() {
        let llm_resp = crate::llm::ChatResponse {
            id: "resp-456".to_string(),
            model: "claude-3".to_string(),
            content: "I'll use tools".to_string(),
            usage: None,
            finish_reason: None,
            tool_calls: vec![crate::llm::ToolCall {
                id: "call-1".to_string(),
                name: "fs_write_file".to_string(),
                arguments: serde_json::json!({"path": "/test", "content": "data"}),
            }],
        };

        let core_resp = LlmServiceImpl::convert_to_core_response(llm_resp);

        assert_eq!(core_resp.tool_calls.len(), 1);
        assert_eq!(core_resp.tool_calls[0].name, "fs_write_file");
        assert_eq!(core_resp.tool_calls[0].id, "call-1");
    }

    // ========================================================================
    // Error Handling Tests
    // ========================================================================

    #[test]
    fn test_get_provider_config_not_found() {
        let service = make_test_service();

        let result = service.get_provider_config("nonexistent-provider");

        assert!(result.is_err());
        assert!(matches!(result, Err(LlmError::Unknown(_))));
    }

    #[test]
    fn test_create_client_fails_for_nonexistent_provider() {
        let service = make_test_service();

        let result = service.create_client("nonexistent-provider");

        assert!(result.is_err());
        assert!(matches!(result, Err(LlmError::Unknown(_))));
    }

    // ========================================================================
    // Integration Tests
    // ========================================================================

    #[test]
    fn test_full_message_conversion_roundtrip() {
        let core_msg = ChatMessage {
            role: CoreMessageRole::User,
            content: MessageContent::Text("Test message".to_string()),
            tool_calls: vec![],
            tool_result: None,
        };

        // Convert to LLM module message
        let llm_msg = LlmServiceImpl::convert_message(core_msg.clone());

        // Convert back to core message
        let back = LlmServiceImpl::convert_to_core_message(llm_msg);

        assert_eq!(back.role, core_msg.role);
        assert_eq!(back.as_text(), core_msg.as_text());
    }

    #[test]
    fn test_get_default_model_for_all_provider_types() {
        let service = make_test_service();

        let providers = vec![
            LlmProvider::OpenAI,
            LlmProvider::Anthropic,
            LlmProvider::Google,
            LlmProvider::Custom {
                name: "custom".to_string(),
            },
        ];

        for provider in providers {
            let model = service.get_default_model(provider.clone());
            assert!(model.is_some(), "Provider {:?} should have a default model", provider);
            assert!(!model.unwrap().is_empty());
        }
    }

    #[test]
    fn test_request_conversion_preserves_all_fields() {
        let core_req = ChatRequest {
            model: "test-model".to_string(),
            messages: vec![
                ChatMessage {
                    role: CoreMessageRole::System,
                    content: MessageContent::Text("System prompt".to_string()),
                    tool_calls: vec![],
                    tool_result: None,
                },
                ChatMessage {
                    role: CoreMessageRole::User,
                    content: MessageContent::Text("User message".to_string()),
                    tool_calls: vec![],
                    tool_result: None,
                },
            ],
            system: Some("Override system".to_string()),
            temperature: Some(0.5),
            max_tokens: Some(500),
            stream: true,
            tools: vec![],
        };

        let llm_req = LlmServiceImpl::convert_request(core_req);

        assert_eq!(llm_req.model, "test-model");
        assert_eq!(llm_req.messages.len(), 2);
        assert_eq!(llm_req.system, Some("Override system".to_string()));
        assert_eq!(llm_req.temperature, Some(0.5));
        assert_eq!(llm_req.max_tokens, Some(500));
        assert_eq!(llm_req.stream, true);
    }
}

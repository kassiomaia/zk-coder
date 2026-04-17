//! Provides the concrete implementation of the `LlmService`.
//!
//! This module contains the `LlmServiceImpl`, which is the concrete implementation
//! of the `LlmService` trait from the core layer. It is responsible for
//! coordinating between the domain entities and the underlying infrastructure,
//! such as clients for `OpenAI` and Anthropic.

mod client_factory;
mod conversions;
mod operations;

use std::sync::Arc;

use crate::core::config::ConfigRepository;

/// The concrete implementation of the LLM service.
///
/// This service is responsible for managing provider clients and providing
/// high-level LLM operations, such as chat completions and API key validation.
pub struct LlmServiceImpl {
    /// The repository for accessing configuration data, such as providers and API keys.
    config_repository: Arc<dyn ConfigRepository>,
}

impl LlmServiceImpl {
    /// Creates a new `LlmServiceImpl`.
    #[must_use]
    pub fn new(config_repository: Arc<dyn ConfigRepository>) -> Self {
        Self { config_repository }
    }
}

#[cfg(test)]
mod tests {
    use super::operations::infer_provider_from_model;
    use super::*;
    use crate::core::config::{ApiType, Provider};
    use crate::core::llm::entities::{
        ChatMessage, ChatRequest, ContentBlock, ImageSource, MessageContent,
        MessageRole as CoreMessageRole, ToolCall, UsageStats,
    };
    use crate::core::llm::services::LlmService;
    use crate::core::llm::{LlmError, LlmProvider};
    use crate::infrastructure::llm::MessageRole as LlmMessageRole;
    use crate::infrastructure::sqlite::repositories::config::SqliteConfigRepository;

    fn make_test_service() -> LlmServiceImpl {
        let config_store =
            SqliteConfigRepository::in_memory().expect("Failed to create config store");
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
        assert!(matches!(
            llm_msg.content,
            crate::core::llm::entities::MessageContent::Text(_)
        ));
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

        assert!(matches!(
            llm_msg.content,
            crate::core::llm::entities::MessageContent::Blocks(_)
        ));
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
            top_p: None,
            stop: None,
            stream: false,
            tools: vec![],
            tool_choice: crate::core::llm::entities::ToolChoiceMode::Auto,
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
            top_p: None,
            stop: None,
            stream: false,
            tools: vec![],
            tool_choice: crate::core::llm::entities::ToolChoiceMode::Auto,
        };

        let llm_req = LlmServiceImpl::convert_request(core_req);

        assert_eq!(
            llm_req.system,
            Some("You are a helpful assistant".to_string())
        );
    }

    #[test]
    fn test_convert_request_with_tools() {
        let core_req = ChatRequest {
            model: "gpt-4".to_string(),
            messages: vec![ChatMessage::user("Use tools")],
            system: None,
            temperature: None,
            max_tokens: None,
            top_p: None,
            stop: None,
            stream: false,
            tools: vec![crate::core::llm::entities::ToolDefinition {
                name: "fs_read_file".to_string(),
                description: "Read a file".to_string(),
                input_schema: serde_json::json!({"type": "object"}),
            }],
            tool_choice: crate::core::llm::entities::ToolChoiceMode::Auto,
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

        assert!(matches!(
            llm_block,
            crate::core::llm::entities::ContentBlock::Text { .. }
        ));
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

        assert!(matches!(
            llm_block,
            crate::core::llm::entities::ContentBlock::Image { .. }
        ));
        if let crate::core::llm::entities::ContentBlock::Image { source } = llm_block {
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
        let llm_resp = crate::core::llm::entities::ChatResponse {
            id: "resp-123".to_string(),
            model: "gpt-4".to_string(),
            content: "Hello there!".to_string(),
            usage: Some(crate::core::llm::entities::UsageStats {
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
        let llm_resp = crate::core::llm::entities::ChatResponse {
            id: "resp-456".to_string(),
            model: "claude-3".to_string(),
            content: "I'll use tools".to_string(),
            usage: None,
            finish_reason: None,
            tool_calls: vec![crate::core::llm::entities::ToolCall {
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
            assert!(
                model.is_some(),
                "Provider {:?} should have a default model",
                provider
            );
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
            top_p: None,
            stop: None,
            stream: true,
            tools: vec![],
            tool_choice: crate::core::llm::entities::ToolChoiceMode::Auto,
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

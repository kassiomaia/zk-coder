//! Comprehensive tests for LLM orchestrator and client factory.
//!
//! Tests the main LLM module functionality including:
//! - Message creation and conversion
//! - Chat request building
//! - Tool definitions
//! - Content blocks
//! - Streaming types
//! - Client factory

#[cfg(test)]
mod tests {
    use super::*;
    use crate::llm::{
        Message, MessageRole, MessageContent, ContentBlock, ImageSource,
        ChatRequest, ChatResponse, LlmEvent, ToolCall, ToolDefinition, ToolResult, UsageStats,
        StreamChunk, StreamDelta, create_client,
    };
    use crate::core::config::ApiType;
    use serde_json::json;

    // =========================================================================
    // Message Creation Tests
    // =========================================================================

    #[test]
    fn test_message_user_creation() {
        let msg = Message::user("Hello, world!");

        assert_eq!(msg.role, MessageRole::User);
        assert_eq!(msg.as_text(), Some("Hello, world!"));
        assert!(msg.tool_calls.is_empty());
        assert!(msg.tool_result.is_none());
    }

    #[test]
    fn test_message_assistant_creation() {
        let msg = Message::assistant("I can help with that.");

        assert_eq!(msg.role, MessageRole::Assistant);
        assert_eq!(msg.as_text(), Some("I can help with that."));
        assert!(msg.tool_calls.is_empty());
        assert!(msg.tool_result.is_none());
    }

    #[test]
    fn test_message_system_creation() {
        let msg = Message::system("You are a helpful assistant.");

        assert_eq!(msg.role, MessageRole::System);
        assert_eq!(msg.as_text(), Some("You are a helpful assistant."));
        assert!(msg.tool_calls.is_empty());
        assert!(msg.tool_result.is_none());
    }

    #[test]
    fn test_message_multimodal_creation() {
        let blocks = vec![
            ContentBlock::text("Hello"),
            ContentBlock::image_base64("image/png", "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNk+M9QDwADhgGAWjR9awAAAABJRU5ErkJggg=="),
        ];

        let msg = Message::multimodal(MessageRole::User, blocks);

        assert_eq!(msg.role, MessageRole::User);
        assert!(msg.as_text().is_none()); // Multimodal messages return None for as_text

        match &msg.content {
            MessageContent::Blocks(blocks) => {
                assert_eq!(blocks.len(), 2);
            }
            _ => panic!("Expected Blocks content"),
        }
    }

    #[test]
    fn test_message_assistant_with_tool_calls() {
        let tool_calls = vec![ToolCall {
            id: "call_123".to_string(),
            name: "search_files".to_string(),
            arguments: json!({"query": "test"}),
        }];

        let msg = Message::assistant_with_tool_calls("Searching...", tool_calls);

        assert_eq!(msg.role, MessageRole::Assistant);
        assert_eq!(msg.as_text(), Some("Searching..."));
        assert_eq!(msg.tool_calls.len(), 1);
        assert_eq!(msg.tool_calls[0].id, "call_123");
        assert_eq!(msg.tool_calls[0].name, "search_files");
        assert!(msg.tool_result.is_none());
    }

    #[test]
    fn test_message_tool_result_creation() {
        let output = json!({"files": ["test.rs", "main.rs"]});
        let msg = Message::tool_result("call_123", output.clone(), false);

        assert_eq!(msg.role, MessageRole::Tool);
        assert!(msg.tool_calls.is_empty());
        assert!(msg.tool_result.is_some());

        let result = msg.tool_result.unwrap();
        assert_eq!(result.call_id, "call_123");
        assert_eq!(result.output, output);
        assert!(!result.is_error);
    }

    #[test]
    fn test_message_tool_result_with_error() {
        let output = json!({"error": "File not found"});
        let msg = Message::tool_result("call_456", output.clone(), true);

        let result = msg.tool_result.unwrap();
        assert_eq!(result.call_id, "call_456");
        assert!(result.is_error);
    }

    #[test]
    fn test_message_empty_text_content() {
        let msg = Message::user("");

        assert_eq!(msg.as_text(), Some(""));
    }

    // =========================================================================
    // Chat Request Builder Tests
    // =========================================================================

    #[test]
    fn test_chat_request_new() {
        let messages = vec![Message::user("Hello")];
        let request = ChatRequest::new("gpt-4", messages);

        assert_eq!(request.model, "gpt-4");
        assert_eq!(request.messages.len(), 1);
        assert!(request.system.is_none());
        assert!(request.temperature.is_none());
        assert!(request.max_tokens.is_none());
        assert!(request.top_p.is_none());
        assert!(!request.stream);
        assert!(request.tools.is_empty());
    }

    #[test]
    fn test_chat_request_with_system() {
        let request = ChatRequest::new("gpt-4", vec![Message::user("Hello")])
            .with_system("You are a helpful assistant.");

        assert_eq!(request.system, Some("You are a helpful assistant.".to_string()));
    }

    #[test]
    fn test_chat_request_with_temperature() {
        let request =
            ChatRequest::new("gpt-4", vec![Message::user("Hello")]).with_temperature(0.7);

        assert_eq!(request.temperature, Some(0.7));
    }

    #[test]
    fn test_chat_request_with_max_tokens() {
        let request =
            ChatRequest::new("gpt-4", vec![Message::user("Hello")]).with_max_tokens(1000);

        assert_eq!(request.max_tokens, Some(1000));
    }

    #[test]
    fn test_chat_request_with_stream() {
        let request = ChatRequest::new("gpt-4", vec![Message::user("Hello")]).with_stream();

        assert!(request.stream);
    }

    #[test]
    fn test_chat_request_builder_chain() {
        let request = ChatRequest::new("gpt-4", vec![Message::user("Hello")])
            .with_system("Be helpful")
            .with_temperature(0.5)
            .with_max_tokens(500)
            .with_stream();

        assert_eq!(request.model, "gpt-4");
        assert_eq!(request.system, Some("Be helpful".to_string()));
        assert_eq!(request.temperature, Some(0.5));
        assert_eq!(request.max_tokens, Some(500));
        assert!(request.stream);
    }

    #[test]
    fn test_chat_request_with_tools() {
        let tools = vec![ToolDefinition {
            name: "read_file".to_string(),
            description: "Read a file".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "path": {"type": "string"}
                }
            }),
        }];

        let request = ChatRequest::new("gpt-4", vec![Message::user("Hello")]).with_tools(tools);

        assert_eq!(request.tools.len(), 1);
        assert_eq!(request.tools[0].name, "read_file");
    }

    #[test]
    fn test_chat_request_with_stop_sequences() {
        let mut request = ChatRequest::new("gpt-4", vec![Message::user("Hello")]);
        request.stop = Some(vec!["\n\n".to_string(), "END".to_string()]);

        assert_eq!(request.stop, Some(vec!["\n\n".to_string(), "END".to_string()]));
    }

    // =========================================================================
    // Content Block Tests
    // =========================================================================

    #[test]
    fn test_content_block_text() {
        let block = ContentBlock::text("Hello, world!");

        match block {
            ContentBlock::Text { text } => {
                assert_eq!(text, "Hello, world!");
            }
            _ => panic!("Expected Text block"),
        }
    }

    #[test]
    fn test_content_block_image_base64() {
        let block = ContentBlock::image_base64("image/jpeg", "/9j/4AAQSkZJRg==");

        match block {
            ContentBlock::Image { source } => {
                assert_eq!(source.source_type, "base64");
                assert_eq!(source.media_type, Some("image/jpeg".to_string()));
                assert_eq!(source.data, "/9j/4AAQSkZJRg==");
            }
            _ => panic!("Expected Image block"),
        }
    }

    #[test]
    fn test_content_block_image_url() {
        let block = ContentBlock::image_url("https://example.com/image.png");

        match block {
            ContentBlock::Image { source } => {
                assert_eq!(source.source_type, "url");
                assert_eq!(source.media_type, None);
                assert_eq!(source.data, "https://example.com/image.png");
            }
            _ => panic!("Expected Image block"),
        }
    }

    #[test]
    fn test_image_source_with_base64() {
        let source = ImageSource {
            source_type: "base64".to_string(),
            media_type: Some("image/png".to_string()),
            data: "fake-base64-data".to_string(),
        };

        assert_eq!(source.source_type, "base64");
        assert_eq!(source.media_type, Some("image/png".to_string()));
    }

    #[test]
    fn test_image_source_with_url() {
        let source = ImageSource {
            source_type: "url".to_string(),
            media_type: None,
            data: "https://example.com/test.jpg".to_string(),
        };

        assert_eq!(source.source_type, "url");
        assert_eq!(source.media_type, None);
    }

    // =========================================================================
    // Message Role Tests
    // =========================================================================

    #[test]
    fn test_message_role_as_str() {
        assert_eq!(MessageRole::System.as_str(), "system");
        assert_eq!(MessageRole::User.as_str(), "user");
        assert_eq!(MessageRole::Assistant.as_str(), "assistant");
        assert_eq!(MessageRole::Tool.as_str(), "tool");
    }

    #[test]
    fn test_message_role_serialization() {
        let role = MessageRole::User;
        let json = serde_json::to_string(&role).unwrap();

        assert_eq!(json, "\"user\"");
    }

    #[test]
    fn test_message_role_deserialization() {
        let json = "\"assistant\"";
        let role: MessageRole = serde_json::from_str(json).unwrap();

        assert_eq!(role, MessageRole::Assistant);
    }

    // =========================================================================
    // Stream Chunk Tests
    // =========================================================================

    #[test]
    fn test_stream_chunk_creation() {
        let chunk = StreamChunk {
            id: "chunk-1".to_string(),
            model: "gpt-4".to_string(),
            delta: Some(StreamDelta {
                role: Some("assistant".to_string()),
                content: Some("Hello".to_string()),
            }),
            usage: None,
            finish_reason: None,
            is_final: false,
        };

        assert_eq!(chunk.id, "chunk-1");
        assert_eq!(chunk.model, "gpt-4");
        assert!(!chunk.is_final);
    }

    #[test]
    fn test_stream_chunk_final() {
        let chunk = StreamChunk {
            id: "chunk-final".to_string(),
            model: "gpt-4".to_string(),
            delta: None,
            usage: Some(UsageStats {
                input_tokens: 10,
                output_tokens: 20,
                total_tokens: 30,
            }),
            finish_reason: Some("stop".to_string()),
            is_final: true,
        };

        assert!(chunk.is_final);
        assert_eq!(chunk.finish_reason, Some("stop".to_string()));
        assert!(chunk.usage.is_some());
    }

    #[test]
    fn test_stream_delta_creation() {
        let delta = StreamDelta {
            role: Some("assistant".to_string()),
            content: Some("Hello, world!".to_string()),
        };

        assert_eq!(delta.role, Some("assistant".to_string()));
        assert_eq!(delta.content, Some("Hello, world!".to_string()));
    }

    #[test]
    fn test_stream_delta_empty() {
        let delta = StreamDelta {
            role: None,
            content: None,
        };

        assert!(delta.role.is_none());
        assert!(delta.content.is_none());
    }

    // =========================================================================
    // Usage Stats Tests
    // =========================================================================

    #[test]
    fn test_usage_stats_creation() {
        let usage = UsageStats {
            input_tokens: 100,
            output_tokens: 200,
            total_tokens: 300,
        };

        assert_eq!(usage.input_tokens, 100);
        assert_eq!(usage.output_tokens, 200);
        assert_eq!(usage.total_tokens, 300);
    }

    #[test]
    fn test_usage_stats_default() {
        let usage = UsageStats {
            input_tokens: 0,
            output_tokens: 0,
            total_tokens: 0,
        };

        assert_eq!(usage.total_tokens, 0);
    }

    // =========================================================================
    // Tool Definition Tests
    // =========================================================================

    #[test]
    fn test_tool_definition_creation() {
        let tool = ToolDefinition {
            name: "search_code".to_string(),
            description: "Search code in the workspace".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "query": {
                        "type": "string",
                        "description": "Search query"
                    }
                },
                "required": ["query"]
            }),
        };

        assert_eq!(tool.name, "search_code");
        assert_eq!(tool.description, "Search code in the workspace");
        assert_eq!(tool.input_schema["type"], "object");
    }

    #[test]
    fn test_tool_definition_serialization() {
        let tool = ToolDefinition {
            name: "read_file".to_string(),
            description: "Read file content".to_string(),
            input_schema: json!({"type": "object"}),
        };

        let json = serde_json::to_string(&tool).unwrap();
        let parsed: ToolDefinition = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.name, "read_file");
    }

    // =========================================================================
    // Tool Call Tests
    // =========================================================================

    #[test]
    fn test_tool_call_creation() {
        let call = ToolCall {
            id: "call_abc123".to_string(),
            name: "run_command".to_string(),
            arguments: json!({"command": "ls -la"}),
        };

        assert_eq!(call.id, "call_abc123");
        assert_eq!(call.name, "run_command");
        assert_eq!(call.arguments["command"], "ls -la");
    }

    #[test]
    fn test_tool_call_serialization() {
        let call = ToolCall {
            id: "call_xyz".to_string(),
            name: "write_file".to_string(),
            arguments: json!({"path": "test.txt", "content": "Hello"}),
        };

        let json = serde_json::to_string(&call).unwrap();
        let parsed: ToolCall = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.id, "call_xyz");
        assert_eq!(parsed.name, "write_file");
    }

    // =========================================================================
    // Tool Result Tests
    // =========================================================================

    #[test]
    fn test_tool_result_creation() {
        let result = ToolResult {
            call_id: "call_123".to_string(),
            output: json!({"success": true}),
            is_error: false,
        };

        assert_eq!(result.call_id, "call_123");
        assert!(!result.is_error);
        assert_eq!(result.output["success"], true);
    }

    #[test]
    fn test_tool_result_error() {
        let result = ToolResult {
            call_id: "call_456".to_string(),
            output: json!({"error": "Command failed"}),
            is_error: true,
        };

        assert!(result.is_error);
    }

    // =========================================================================
    // Chat Response Tests
    // =========================================================================

    #[test]
    fn test_chat_response_creation() {
        let response = ChatResponse {
            id: "resp-1".to_string(),
            model: "gpt-4".to_string(),
            content: "Hello, how can I help?".to_string(),
            usage: Some(UsageStats {
                input_tokens: 10,
                output_tokens: 5,
                total_tokens: 15,
            }),
            finish_reason: Some("stop".to_string()),
            tool_calls: vec![],
        };

        assert_eq!(response.id, "resp-1");
        assert_eq!(response.model, "gpt-4");
        assert_eq!(response.content, "Hello, how can I help?");
        assert!(response.usage.is_some());
        assert_eq!(response.finish_reason, Some("stop".to_string()));
        assert!(response.tool_calls.is_empty());
    }

    #[test]
    fn test_chat_response_with_tool_calls() {
        let response = ChatResponse {
            id: "resp-2".to_string(),
            model: "gpt-4".to_string(),
            content: "I'll search for that.".to_string(),
            usage: None,
            finish_reason: Some("tool_calls".to_string()),
            tool_calls: vec![ToolCall {
                id: "call_1".to_string(),
                name: "search".to_string(),
                arguments: json!({"query": "test"}),
            }],
        };

        assert_eq!(response.tool_calls.len(), 1);
        assert_eq!(response.finish_reason, Some("tool_calls".to_string()));
    }

    // =========================================================================
    // LLM Event Tests
    // =========================================================================

    #[test]
    fn test_llm_event_started() {
        let event = LlmEvent::Started {
            request_id: "req-123".to_string(),
            model: "gpt-4".to_string(),
        };

        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("\"type\":\"started\""));
        assert!(json.contains("req-123"));
    }

    #[test]
    fn test_llm_event_chunk() {
        let event = LlmEvent::Chunk {
            request_id: "req-456".to_string(),
            delta: "Hello, world!".to_string(),
        };

        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("\"type\":\"chunk\""));
        assert!(json.contains("Hello, world!"));
    }

    #[test]
    fn test_llm_event_complete() {
        let event = LlmEvent::Complete {
            request_id: "req-789".to_string(),
            content: "Full response here".to_string(),
            usage: Some(UsageStats {
                input_tokens: 10,
                output_tokens: 20,
                total_tokens: 30,
            }),
            finish_reason: Some("stop".to_string()),
        };

        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("\"type\":\"complete\""));
        assert!(json.contains("Full response here"));
    }

    #[test]
    fn test_llm_event_error() {
        let event = LlmEvent::Error {
            request_id: "req-error".to_string(),
            error: "API key invalid".to_string(),
            retryable: false,
        };

        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("\"type\":\"error\""));
        assert!(json.contains("API key invalid"));
        assert!(json.contains("\"retryable\":false"));
    }

    // =========================================================================
    // Message Content Tests
    // =========================================================================

    #[test]
    fn test_message_content_text() {
        let content = MessageContent::Text("Hello".to_string());

        assert!(matches!(content, MessageContent::Text(_)));
    }

    #[test]
    fn test_message_content_blocks() {
        let blocks = vec![
            ContentBlock::text("Hello"),
            ContentBlock::text("World"),
        ];
        let content = MessageContent::Blocks(blocks);

        assert!(matches!(content, MessageContent::Blocks(_)));
    }

    #[test]
    fn test_message_content_serialization_text() {
        let content = MessageContent::Text("Hello".to_string());
        let json = serde_json::to_string(&content).unwrap();

        assert_eq!(json, "\"Hello\"");
    }

    #[test]
    fn test_message_content_serialization_blocks() {
        let blocks = vec![ContentBlock::text("Hello")];
        let content = MessageContent::Blocks(blocks);

        let json = serde_json::to_string(&content).unwrap();
        assert!(json.contains("\"text\""));
    }

    // =========================================================================
    // Client Factory Tests
    // =========================================================================

    #[test]
    fn test_create_client_openai() {
        let client = create_client(
            ApiType::OpenAI,
            "test-key".to_string(),
            None,
        );

        assert_eq!(client.provider_type(), ApiType::OpenAI);
    }

    #[test]
    fn test_create_client_anthropic() {
        let client = create_client(
            ApiType::Anthropic,
            "test-key".to_string(),
            None,
        );

        assert_eq!(client.provider_type(), ApiType::Anthropic);
    }

    #[test]
    fn test_create_client_google() {
        let client = create_client(
            ApiType::Google,
            "test-key".to_string(),
            None,
        );

        assert_eq!(client.provider_type(), ApiType::Google);
    }

    #[test]
    fn test_create_client_with_custom_url() {
        let client = create_client(
            ApiType::OpenAI,
            "test-key".to_string(),
            Some("https://custom.api.com".to_string()),
        );

        assert_eq!(client.provider_type(), ApiType::OpenAI);
    }

    // =========================================================================
    // MessageContent Edge Cases
    // =========================================================================

    #[test]
    fn test_message_with_empty_blocks() {
        let msg = Message::multimodal(MessageRole::User, vec![]);

        match msg.content {
            MessageContent::Blocks(blocks) => {
                assert!(blocks.is_empty());
            }
            _ => panic!("Expected empty Blocks"),
        }
    }

    #[test]
    fn test_message_with_mixed_content_blocks() {
        let blocks = vec![
            ContentBlock::text("Check this image:"),
            ContentBlock::image_url("https://example.com/chart.png"),
            ContentBlock::text("What do you think?"),
        ];
        let msg = Message::multimodal(MessageRole::User, blocks);

        match msg.content {
            MessageContent::Blocks(blocks) => {
                assert_eq!(blocks.len(), 3);
            }
            _ => panic!("Expected Blocks"),
        }
    }

    // =========================================================================
    // Stream Chunk Edge Cases
    // =========================================================================

    #[test]
    fn test_stream_chunk_with_no_delta() {
        let chunk = StreamChunk {
            id: "chunk-no-delta".to_string(),
            model: "gpt-4".to_string(),
            delta: None,
            usage: None,
            finish_reason: None,
            is_final: false,
        };

        assert!(chunk.delta.is_none());
        assert!(!chunk.is_final);
    }

    #[test]
    fn test_stream_chunk_with_empty_delta() {
        let chunk = StreamChunk {
            id: "chunk-empty-delta".to_string(),
            model: "gpt-4".to_string(),
            delta: Some(StreamDelta {
                role: None,
                content: Some("".to_string()),
            }),
            usage: None,
            finish_reason: None,
            is_final: false,
        };

        assert_eq!(chunk.delta.as_ref().unwrap().content, Some("".to_string()));
    }
}

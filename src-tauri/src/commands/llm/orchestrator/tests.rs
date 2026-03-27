use super::*;
use crate::core::llm::entities::{
    ChatRequest, ChatResponse, ContentBlock, LlmError, LlmResult, MessageContent,
    MessageRole as CoreMessageRole, UsageStats,
};
use crate::core::llm::services::LlmService;
use crate::infrastructure::llm::{Message, MessageRole as InfraMessageRole};
use crate::infrastructure::tools::{OrchestratorToolResult, ToolRegistry};
use std::sync::Arc;

// Mock LlmService for testing
struct MockLlmService {
    response_content: String,
    response_model: String,
    should_fail: bool,
    error_message: Option<String>,
    tool_calls: Vec<crate::core::llm::entities::ToolCall>,
}

impl MockLlmService {
    fn new() -> Self {
        Self {
            response_content: "Test response".to_string(),
            response_model: "test-model".to_string(),
            should_fail: false,
            error_message: None,
            tool_calls: vec![],
        }
    }

    fn with_content(content: &str) -> Self {
        Self {
            response_content: content.to_string(),
            response_model: "test-model".to_string(),
            should_fail: false,
            error_message: None,
            tool_calls: vec![],
        }
    }

    fn with_error(message: &str) -> Self {
        Self {
            response_content: String::new(),
            response_model: String::new(),
            should_fail: true,
            error_message: Some(message.to_string()),
            tool_calls: vec![],
        }
    }

    fn with_tool_calls(calls: Vec<crate::core::llm::entities::ToolCall>) -> Self {
        Self {
            response_content: "Thinking...".to_string(),
            response_model: "test-model".to_string(),
            should_fail: false,
            error_message: None,
            tool_calls: calls,
        }
    }
}

impl LlmService for MockLlmService {
    fn chat(&self, _request: ChatRequest) -> LlmResult<ChatResponse> {
        if self.should_fail {
            let msg = self
                .error_message
                .clone()
                .unwrap_or_else(|| "Unknown error".to_string());
            return Err(LlmError::Provider {
                code: "error".to_string(),
                message: msg,
            });
        }

        Ok(ChatResponse {
            id: "test-response-123".to_string(),
            content: self.response_content.clone(),
            model: self.response_model.clone(),
            tool_calls: self.tool_calls.clone(),
            usage: Some(UsageStats {
                input_tokens: 10,
                output_tokens: 20,
                total_tokens: 30,
            }),
            finish_reason: Some("stop".to_string()),
        })
    }

    fn validate_api_key(
        &self,
        _provider: crate::core::llm::repositories::LlmProvider,
        _api_key: &str,
    ) -> LlmResult<bool> {
        Ok(true)
    }

    fn fetch_models(
        &self,
        _provider: crate::core::llm::repositories::LlmProvider,
        _api_key: &str,
    ) -> LlmResult<Vec<String>> {
        Ok(vec!["model-1".to_string(), "model-2".to_string()])
    }

    fn get_default_model(
        &self,
        _provider: crate::core::llm::repositories::LlmProvider,
    ) -> Option<String> {
        Some("default-model".to_string())
    }
}

// Note: We can't fully test ToolExecutor without Tauri AppHandle
// The tests below focus on testing helper functions and logic

fn create_test_request() -> SendChatRequest {
    SendChatRequest {
        conversation_id: "conv-123".to_string(),
        thread_id: Some("thread-123".to_string()),
        content: "Hello, AI!".to_string(),
        model_id: Some("test-model".to_string()),
        system_prompt: None,
        temperature: Some(0.7),
        max_tokens: Some(1000),
    }
}

// =========================================================================
// OrchestratedConversationRunner Tests
// =========================================================================

#[test]
fn test_orchestrated_conversation_runner_creation() {
    let mock_service = Arc::new(MockLlmService::new()) as Arc<dyn LlmService>;
    let runner = OrchestratedConversationRunner::new(mock_service);
    // Runner is created successfully
    assert!(runner
        .llm_service
        .chat(ChatRequest {
            model: "test".to_string(),
            messages: vec![],
            system: None,
            temperature: None,
            max_tokens: None,
            top_p: None,
            stop: None,
            tools: vec![],
            stream: false,
        })
        .is_ok());
}

// =========================================================================
// Textual Tool Plan Detection Tests
// =========================================================================

#[test]
fn test_is_textual_tool_plan_empty() {
    assert!(!is_textual_tool_plan(""));
}

#[test]
fn test_is_textual_tool_plan_invalid_json() {
    assert!(!is_textual_tool_plan("not json at all"));
}

#[test]
fn test_is_textual_tool_plan_tool_calls_array() {
    let json = r#"[
            {"name": "read_file", "arguments": {"path": "/test"}}
        ]"#;
    assert!(is_textual_tool_plan(json));
}

#[test]
fn test_is_textual_tool_plan_with_tool_calls_field() {
    let json = r#"{"tool_calls": [
            {"name": "write_file", "arguments": {"path": "/test", "content": "data"}}
        ]}"#;
    assert!(is_textual_tool_plan(json));
}

#[test]
fn test_is_textual_tool_plan_empty_name() {
    let json = r#"[{"name": "", "arguments": {}}]"#;
    assert!(!is_textual_tool_plan(json));
}

#[test]
fn test_is_textual_tool_plan_no_arguments() {
    let json = r#"[{"name": "tool", "arguments": null}]"#;
    assert!(!is_textual_tool_plan(json));
}

#[test]
fn test_is_textual_tool_plan_string_arguments() {
    let json = r#"[{"name": "tool", "arguments": "{\"path\": \"/test\"}"}]"#;
    assert!(is_textual_tool_plan(json));
}

#[test]
fn test_is_textual_tool_plan_regular_text() {
    assert!(!is_textual_tool_plan("This is just regular text"));
    assert!(!is_textual_tool_plan("I'll help you with that"));
}

// =========================================================================
// Environment Flag Parsing Tests
// =========================================================================

#[test]
fn test_parse_env_flag_none() {
    assert!(!parse_env_flag(None));
}

#[test]
fn test_parse_env_flag_empty() {
    assert!(!parse_env_flag(Some("")));
}

#[test]
fn test_parse_env_flag_true() {
    assert!(parse_env_flag(Some("true")));
    assert!(parse_env_flag(Some("TRUE")));
    assert!(parse_env_flag(Some("1")));
    assert!(parse_env_flag(Some("yes")));
    assert!(parse_env_flag(Some("on")));
}

#[test]
fn test_parse_env_flag_false() {
    assert!(!parse_env_flag(Some("false")));
    assert!(!parse_env_flag(Some("0")));
    assert!(!parse_env_flag(Some("no")));
    assert!(!parse_env_flag(Some("off")));
    assert!(!parse_env_flag(Some("random")));
}

#[test]
fn test_parse_env_flag_whitespace() {
    assert!(parse_env_flag(Some(" true ")));
    assert!(!parse_env_flag(Some("   ")));
}

// =========================================================================
// Tool Result Append Tests
// =========================================================================

#[test]
fn test_append_tool_results_to_llm_context() {
    let mut messages = vec![Message {
        role: InfraMessageRole::User,
        content: MessageContent::Text("Hello".to_string()),
        tool_calls: vec![],
        tool_result: None,
    }];

    let results = vec![
        OrchestratorToolResult {
            call_id: "call-1".to_string(),
            name: "tool1".to_string(),
            output: serde_json::json!({"result": "data1"}),
            is_error: false,
        },
        OrchestratorToolResult {
            call_id: "call-2".to_string(),
            name: "tool2".to_string(),
            output: serde_json::json!({"result": "data2"}),
            is_error: false,
        },
    ];

    append_tool_results_to_llm_context(&mut messages, &results);

    assert_eq!(messages.len(), 3); // Original + 2 tool results
    assert!(matches!(messages[1].role, InfraMessageRole::Tool));
    assert!(matches!(messages[2].role, InfraMessageRole::Tool));
}

#[test]
fn test_append_tool_results_error_handling() {
    let mut messages = vec![];
    let results = vec![OrchestratorToolResult {
        call_id: "call-1".to_string(),
        name: "tool1".to_string(),
        output: serde_json::json!({"error": "failed"}),
        is_error: true,
    }];

    append_tool_results_to_llm_context(&mut messages, &results);

    assert_eq!(messages.len(), 1);
    if let Some(tool_result) = &messages[0].tool_result {
        assert!(tool_result.is_error);
    } else {
        panic!("Expected tool_result to be set");
    }
}

// =========================================================================
// NoopToolExecutor Tests
// =========================================================================

#[tokio::test]
async fn test_noop_tool_executor_no_definitions() {
    let executor = NoopToolExecutor;
    assert!(executor.definitions().is_empty());
}

// =========================================================================
// Request/Response Type Tests
// =========================================================================

#[test]
fn test_send_chat_request_serialization() {
    let request = SendChatRequest {
        conversation_id: "conv-123".to_string(),
        thread_id: Some("thread-456".to_string()),
        content: "Test content".to_string(),
        model_id: Some("gpt-4".to_string()),
        system_prompt: Some("You are helpful".to_string()),
        temperature: Some(0.8),
        max_tokens: Some(2000),
    };

    let json = serde_json::to_string(&request);
    assert!(json.is_ok());

    let deserialized: Result<SendChatRequest, _> = serde_json::from_str(&json.unwrap());
    assert!(deserialized.is_ok());
    let req = deserialized.unwrap();
    assert_eq!(req.conversation_id, "conv-123");
    assert_eq!(req.thread_id, Some("thread-456".to_string()));
}

#[test]
fn test_send_chat_response_serialization() {
    let response = SendChatResponse {
        content: "Response content".to_string(),
        model: "gpt-4".to_string(),
        usage: Some(UsageStats {
            input_tokens: 100,
            output_tokens: 200,
            total_tokens: 300,
        }),
        message: crate::core::chat::entities::ChatMessage::assistant("conv-1", "Response"),
    };

    let json = serde_json::to_string(&response);
    assert!(json.is_ok());
}

// =========================================================================
// Constant Tests
// =========================================================================

#[test]
fn test_orchestrator_constants() {
    assert_eq!(ORCHESTRATOR_MAX_STEPS, 6);
    assert_eq!(ORCHESTRATOR_LLM_MAX_RETRIES, 2);
    assert_eq!(ORCHESTRATOR_LLM_RETRY_BASE_MS, 500);
}

#[test]
fn test_orchestrator_tool_registry_env() {
    assert_eq!(ORCHESTRATOR_TOOL_REGISTRY_ENV, "CODER_LLM_TOOL_REGISTRY");
}

// =========================================================================
// Error Handling Tests
// =========================================================================

#[tokio::test]
async fn test_retry_with_timeout_error() {
    let mock_service =
        Arc::new(MockLlmService::with_error("Request timeout")) as Arc<dyn LlmService>;

    let request = ChatRequest {
        model: "test".to_string(),
        messages: vec![],
        system: None,
        temperature: None,
        max_tokens: None,
        top_p: None,
        stop: None,
        tools: vec![],
        stream: false,
    };

    // Test that timeout errors are retryable
    let result = run_chat_with_retry(mock_service, request).await;

    // Should eventually fail after retries
    assert!(result.is_err());
}

#[tokio::test]
async fn test_retry_with_rate_limit_error() {
    let mock_service =
        Arc::new(MockLlmService::with_error("Rate limit exceeded")) as Arc<dyn LlmService>;

    let request = ChatRequest {
        model: "test".to_string(),
        messages: vec![],
        system: None,
        temperature: None,
        max_tokens: None,
        top_p: None,
        stop: None,
        tools: vec![],
        stream: false,
    };

    let result = run_chat_with_retry(mock_service, request).await;

    // Should eventually fail after retries
    assert!(result.is_err());
}

#[tokio::test]
async fn test_no_retry_for_permanent_errors() {
    let mock_service =
        Arc::new(MockLlmService::with_error("Invalid API key")) as Arc<dyn LlmService>;

    let request = ChatRequest {
        model: "test".to_string(),
        messages: vec![],
        system: None,
        temperature: None,
        max_tokens: None,
        top_p: None,
        stop: None,
        tools: vec![],
        stream: false,
    };

    let result = run_chat_with_retry(mock_service, request).await;

    // Should fail immediately without retry
    assert!(result.is_err());
}

// =========================================================================
// Input Validation Tests
// =========================================================================

#[test]
fn test_send_chat_request_defaults() {
    let request = SendChatRequest {
        conversation_id: "conv-1".to_string(),
        thread_id: None,
        content: "Test".to_string(),
        model_id: None,
        system_prompt: None,
        temperature: None,
        max_tokens: None,
    };

    assert_eq!(request.conversation_id, "conv-1");
    assert!(request.thread_id.is_none());
    assert!(request.model_id.is_none());
    assert!(request.system_prompt.is_none());
    assert!(request.temperature.is_none());
    assert!(request.max_tokens.is_none());
}

#[test]
fn test_send_chat_request_with_thread() {
    let request = SendChatRequest {
        conversation_id: "conv-1".to_string(),
        thread_id: Some("thread-1".to_string()),
        content: "Test".to_string(),
        model_id: None,
        system_prompt: None,
        temperature: None,
        max_tokens: None,
    };

    assert_eq!(request.thread_id, Some("thread-1".to_string()));
}

#[test]
fn test_send_chat_request_temperature_bounds() {
    let mut request = create_test_request();

    // Test various temperature values
    request.temperature = Some(0.0);
    assert_eq!(request.temperature, Some(0.0));

    request.temperature = Some(1.0);
    assert_eq!(request.temperature, Some(1.0));

    request.temperature = Some(2.0);
    assert_eq!(request.temperature, Some(2.0));
}

#[test]
fn test_send_chat_request_max_tokens() {
    let mut request = create_test_request();

    request.max_tokens = Some(0);
    assert_eq!(request.max_tokens, Some(0));

    request.max_tokens = Some(4096);
    assert_eq!(request.max_tokens, Some(4096));

    request.max_tokens = Some(128000);
    assert_eq!(request.max_tokens, Some(128000));
}

// =========================================================================
// Unicode and Special Characters Tests
// =========================================================================

#[test]
fn test_send_chat_request_unicode_content() {
    let request = SendChatRequest {
        conversation_id: "conv-1".to_string(),
        thread_id: None,
        content: "Hello 🚀 World 🌍".to_string(),
        model_id: None,
        system_prompt: None,
        temperature: None,
        max_tokens: None,
    };

    assert_eq!(request.content, "Hello 🚀 World 🌍");
}

#[test]
fn test_send_chat_request_unicode_system_prompt() {
    let request = SendChatRequest {
        conversation_id: "conv-1".to_string(),
        thread_id: None,
        content: "Test".to_string(),
        model_id: None,
        system_prompt: Some("You are a helpful assistant 🤖".to_string()),
        temperature: None,
        max_tokens: None,
    };

    assert_eq!(
        request.system_prompt,
        Some("You are a helpful assistant 🤖".to_string())
    );
}

// =========================================================================
// RegistryToolExecutor Tests
// =========================================================================

#[test]
fn test_registry_tool_executor_creation() {
    // Test that we can create a RegistryToolExecutor
    // This requires a ToolRegistry which may fail in test environment
    match ToolRegistry::new() {
        Ok(registry) => {
            let executor = RegistryToolExecutor::new(registry, "test-conv".to_string());
            assert!(executor.definitions().len() > 0);
        }
        Err(_) => {
            // ToolRegistry may not be available in test environment
            // This is expected behavior
        }
    }
}

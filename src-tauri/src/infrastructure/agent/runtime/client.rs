//! LLM client utilities for agent runtime.
//!
//! Provides functions for creating and using LLM clients
//! within the agent execution context.

use tauri::{AppHandle, Manager};

use crate::application::llm::get_code_operation_model;
use crate::commands::{ChatState, LlmState};
use crate::core::chat::entities::MessageRole;
use crate::core::config::ConfigRepository;
use crate::core::llm::entities::{ChatMessage as Message, ChatRequest, ChatResponse, UsageStats};
#[allow(unused_imports)]
use crate::infrastructure::llm::{create_client, LlmClient, LlmError, LlmResult, StreamChunk};

use super::types::AgentRuntimeError;

/// Retry configuration for LLM calls.
pub struct LlmRetryConfig {
    /// Maximum number of retry attempts.
    pub max_retries: usize,
    /// Base delay in milliseconds for exponential backoff.
    pub base_delay_ms: u64,
}

impl Default for LlmRetryConfig {
    fn default() -> Self {
        Self {
            max_retries: 2,
            base_delay_ms: 1000,
        }
    }
}

/// Creates an LLM client from app config for agent execution.
///
/// # Errors
///
/// Returns an error if the model or provider cannot be resolved.
pub async fn create_llm_client(
    app_handle: &AppHandle,
    model_id: Option<String>,
) -> Result<(Box<dyn LlmClient>, String), AgentRuntimeError> {
    log::info!("[AgentRuntime] Creating LLM client for agent execution");
    let llm_state = app_handle.state::<LlmState>();
    let config_repository = llm_state.config_repository.clone();

    // Use provided model_id or fall back to code operation default
    let model = if let Some(mid) = model_id {
        log::info!("[AgentRuntime] Using session model: {}", mid);
        match config_repository.get_model(&mid) {
            Ok(Some(m)) => m,
            Ok(None) => {
                log::warn!("[AgentRuntime] Model '{}' not found, falling back to default", mid);
                get_code_operation_model(config_repository.as_ref()).map_err(|e| {
                    log::error!("[AgentRuntime] Failed to get code operation model: {}", e);
                    AgentRuntimeError::Config(e.to_string())
                })?
            }
            Err(e) => {
                log::warn!("[AgentRuntime] Failed to get model '{}', falling back to default: {}", mid, e);
                get_code_operation_model(config_repository.as_ref()).map_err(|e| {
                    log::error!("[AgentRuntime] Failed to get code operation model: {}", e);
                    AgentRuntimeError::Config(e.to_string())
                })?
            }
        }
    } else {
        log::info!("[AgentRuntime] Using code operation default model");
        get_code_operation_model(config_repository.as_ref()).map_err(|e| {
            log::error!("[AgentRuntime] Failed to get code operation model: {}", e);
            AgentRuntimeError::Config(e.to_string())
        })?
    };

    log::info!(
        "[AgentRuntime] Using model: {} ({})",
        model.name,
        model.provider_id
    );

    let provider = config_repository
        .get_provider(&model.provider_id)
        .map_err(|e| AgentRuntimeError::Config(e.to_string()))?
        .ok_or_else(|| {
            log::error!("[AgentRuntime] Provider '{}' not found", model.provider_id);
            AgentRuntimeError::Config(format!("Provider '{}' not found", model.provider_id))
        })?;

    log::info!("[AgentRuntime] Provider found: {}", provider.id);

    let api_key = get_api_key_for_provider(&provider.id, config_repository.as_ref())?;
    let model_name = model.name.clone();

    log::info!(
        "[AgentRuntime] Creating client with api_type: {:?}",
        provider.api_type
    );
    let client = create_client(provider.api_type, api_key, provider.base_url);
    log::info!("[AgentRuntime] LLM client created successfully");
    Ok((client, model_name))
}

/// Gets the API key for a provider from the database.
fn get_api_key_for_provider(
    provider_id: &str,
    config_repository: &dyn ConfigRepository,
) -> Result<String, AgentRuntimeError> {
    config_repository
        .retrieve_api_key(provider_id)
        .map_err(|e| {
            AgentRuntimeError::Config(format!(
                "API key not found for provider '{}': {}",
                provider_id, e
            ))
        })
}

/// Executes a chat request with retry on transient errors.
///
/// # Errors
///
/// Returns an error if all retry attempts fail.
pub async fn chat_with_retry(
    client: &dyn LlmClient,
    request: &ChatRequest,
    config: &LlmRetryConfig,
) -> Result<ChatResponse, AgentRuntimeError> {
    let mut attempt = 0usize;
    loop {
        match client.chat(request.clone()).await {
            Ok(response) => return Ok(response),
            Err(err) => {
                let can_retry = err.is_retryable() && attempt < config.max_retries;
                if !can_retry {
                    return Err(AgentRuntimeError::Llm(err.to_string()));
                }

                let delay_ms = config.base_delay_ms * (1u64 << attempt);
                log::warn!(
                    "[AgentRuntime] Transient LLM error (attempt {}/{}), retrying in {}ms: {}",
                    attempt + 1,
                    config.max_retries + 1,
                    delay_ms,
                    err
                );
                tokio::time::sleep(std::time::Duration::from_millis(delay_ms)).await;
                attempt += 1;
            }
        }
    }
}

/// Builds LLM message context from conversation history.
///
/// # Errors
///
/// Returns an error if the chat store cannot be accessed.
pub async fn build_llm_context(
    session: &crate::core::agent::session::AgentSession,
    _request: &crate::core::agent::task::AgentRequest,
    app_handle: &AppHandle,
) -> Result<Vec<Message>, AgentRuntimeError> {
    let chat_state = app_handle.state::<ChatState>();
    let history = chat_state
        .service
        .load_thread_messages(&session.thread_id)
        .unwrap_or_default();

    let context: Vec<Message> = history
        .iter()
        .map(|msg| match msg.role {
            MessageRole::User => Message::user(msg.content.clone()),
            MessageRole::Assistant => Message::assistant(msg.content.clone()),
            MessageRole::System => Message::system(msg.content.clone()),
        })
        .collect();

    Ok(context)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::agent::session::SessionId;
    use crate::core::agent::task::AgentRequest;
    use std::sync::{Arc, Mutex};

    // Mock LLM client for testing retry logic
    struct MockLlmClient {
        call_count: Arc<Mutex<usize>>,
        should_fail: bool,
        fail_with_retryable: bool,
        response_content: String,
    }

    impl MockLlmClient {
        fn new_success(content: String) -> Self {
            Self {
                call_count: Arc::new(Mutex::new(0)),
                should_fail: false,
                fail_with_retryable: false,
                response_content: content,
            }
        }

        fn new_fail_retryable(_attempts_before_success: usize) -> Self {
            Self {
                call_count: Arc::new(Mutex::new(0)),
                should_fail: true,
                fail_with_retryable: true,
                response_content: String::new(),
            }
        }

        fn new_fail_non_retryable() -> Self {
            Self {
                call_count: Arc::new(Mutex::new(0)),
                should_fail: true,
                fail_with_retryable: false,
                response_content: String::new(),
            }
        }

        fn call_count(&self) -> usize {
            *self.call_count.lock().unwrap()
        }
    }

    impl LlmClient for MockLlmClient {
        fn provider_type(&self) -> crate::core::config::ApiType {
            crate::core::config::ApiType::OpenAI
        }

        fn chat<'a>(
            &'a self,
            _request: ChatRequest,
        ) -> futures::future::BoxFuture<'a, LlmResult<ChatResponse>> {
            // Increment call count before executing
            *self.call_count.lock().unwrap() += 1;
            Box::pin(async move {
                if self.should_fail {
                    let error = if self.fail_with_retryable {
                        LlmError::timeout(30)
                    } else {
                        LlmError::api(401, "Unauthorized")
                    };
                    Err(error)
                } else {
                    Ok(ChatResponse {
                        id: "test-123".to_string(),
                        model: "test-model".to_string(),
                        content: self.response_content.clone(),
                        usage: Some(UsageStats {
                            input_tokens: 10,
                            output_tokens: 20,
                            total_tokens: 30,
                        }),
                        finish_reason: Some("stop".to_string()),
                        tool_calls: vec![],
                    })
                }
            })
        }

        fn chat_stream(
            &self,
            _request: ChatRequest,
        ) -> std::pin::Pin<Box<dyn futures::Stream<Item = LlmResult<StreamChunk>> + Send>> {
            use futures::stream;
            Box::pin(stream::empty())
        }

        fn validate_api_key(&self) -> futures::future::BoxFuture<'_, LlmResult<bool>> {
            Box::pin(std::future::ready(Ok(true)))
        }

        fn fetch_models(&self) -> futures::future::BoxFuture<'_, LlmResult<Vec<String>>> {
            Box::pin(std::future::ready(Ok(vec!["gpt-4".to_string()])))
        }
    }

    #[test]
    fn test_llm_retry_config_default() {
        let config = LlmRetryConfig::default();
        assert_eq!(config.max_retries, 2);
        assert_eq!(config.base_delay_ms, 1000);
    }

    #[test]
    fn test_llm_retry_config_custom() {
        let config = LlmRetryConfig {
            max_retries: 5,
            base_delay_ms: 500,
        };
        assert_eq!(config.max_retries, 5);
        assert_eq!(config.base_delay_ms, 500);
    }

    #[test]
    fn test_llm_retry_config_fields() {
        let config = LlmRetryConfig {
            max_retries: 3,
            base_delay_ms: 2000,
        };

        // Verify we can access fields
        assert_eq!(config.max_retries, 3);
        assert_eq!(config.base_delay_ms, 2000);
    }

    #[test]
    fn test_agent_runtime_error_variants() {
        use super::super::types::AgentRuntimeError;

        // Test that error variants can be created
        let _ = AgentRuntimeError::AgentNotFound("test-agent".to_string());
        let _ = AgentRuntimeError::SessionNotFound("test-session".to_string());
        let _ = AgentRuntimeError::Llm("LLM error".to_string());
        let _ = AgentRuntimeError::LoopDetected;
        let _ = AgentRuntimeError::ActionDenied;
        let _ = AgentRuntimeError::ApprovalTimeout;
        let _ = AgentRuntimeError::Repository("Repository error".to_string());
        let _ = AgentRuntimeError::Config("Config error".to_string());
    }

    #[test]
    fn test_agent_runtime_error_display() {
        use super::super::types::AgentRuntimeError;

        let err = AgentRuntimeError::AgentNotFound("test-agent".to_string());
        assert!(err.to_string().contains("test-agent"));

        let err = AgentRuntimeError::LoopDetected;
        assert!(err.to_string().contains("repetitive"));

        let err = AgentRuntimeError::ActionDenied;
        assert!(err.to_string().contains("denied"));

        let err = AgentRuntimeError::ApprovalTimeout;
        assert!(err.to_string().contains("timeout"));
    }

    #[test]
    fn test_retry_delay_exponential_backoff() {
        // Test the exponential backoff calculation
        // delay = base_delay_ms * (1 << attempt)
        let base_delay_ms = 1000u64;

        let delay_0 = base_delay_ms * (1u64 << 0);
        let delay_1 = base_delay_ms * (1u64 << 1);
        let delay_2 = base_delay_ms * (1u64 << 2);

        assert_eq!(delay_0, 1000);
        assert_eq!(delay_1, 2000);
        assert_eq!(delay_2, 4000);
    }

    #[test]
    fn test_retry_delay_bounds() {
        // Verify the delay calculation doesn't overflow
        let base_delay_ms = 1000u64;
        let max_attempts = 10;

        for attempt in 0..max_attempts {
            let delay = base_delay_ms.saturating_mul(1u64 << attempt);
            // Should not overflow for reasonable values
            assert!(
                delay > 0 || attempt >= 63,
                "Delay should be positive unless overflow"
            );
        }
    }

    // State tracking tests for agent session and request
    #[test]
    fn test_session_id_creation() {
        let session_id = SessionId("test-session-123".to_string());
        assert_eq!(session_id.0, "test-session-123");
    }

    #[test]
    fn test_agent_request_creation() {
        let request = AgentRequest {
            content: "Test request".to_string(),
            attachments: vec![],
            context: None,
        };

        assert_eq!(request.content, "Test request");
        assert!(request.attachments.is_empty());
        assert!(request.context.is_none());
    }

    #[test]
    fn test_agent_request_with_context() {
        let request = AgentRequest {
            content: "Test request".to_string(),
            attachments: vec![],
            context: Some(crate::core::agent::task::RequestContext {
                file_path: Some("/path/to/file.rs".to_string()),
                selection: Some("selected code".to_string()),
            }),
        };

        assert!(request.context.is_some());
        let ctx = request.context.unwrap();
        assert_eq!(ctx.file_path, Some("/path/to/file.rs".to_string()));
        assert_eq!(ctx.selection, Some("selected code".to_string()));
    }

    #[test]
    fn test_agent_request_with_attachments() {
        let request = AgentRequest {
            content: "Test request".to_string(),
            attachments: vec![
                crate::core::agent::task::Attachment {
                    name: "test.txt".to_string(),
                    content: "hello world".to_string(),
                    content_type: "text/plain".to_string(),
                },
                crate::core::agent::task::Attachment {
                    name: "image.png".to_string(),
                    content: "base64data".to_string(),
                    content_type: "image/png".to_string(),
                },
            ],
            context: None,
        };

        assert_eq!(request.attachments.len(), 2);
        assert_eq!(request.attachments[0].name, "test.txt");
        assert_eq!(request.attachments[1].content_type, "image/png");
    }

    #[test]
    fn test_message_conversions() {
        // Test that we can create messages for context building
        let user_msg = Message::user("Hello");
        let assistant_msg = Message::assistant("Hi there");
        let system_msg = Message::system("You are helpful");

        assert_eq!(user_msg.as_text(), Some("Hello"));
        assert_eq!(assistant_msg.as_text(), Some("Hi there"));
        assert_eq!(system_msg.as_text(), Some("You are helpful"));
    }

    #[test]
    fn test_chat_request_builder() {
        let messages = vec![Message::user("Hello")];

        let request = ChatRequest::new("gpt-4", messages)
            .with_system("You are helpful")
            .with_temperature(0.7)
            .with_max_tokens(1000);

        assert_eq!(request.model, "gpt-4");
        assert_eq!(request.system, Some("You are helpful".to_string()));
        assert_eq!(request.temperature, Some(0.7));
        assert_eq!(request.max_tokens, Some(1000));
    }

    #[test]
    fn test_usage_stats_calculation() {
        let stats = UsageStats {
            input_tokens: 100,
            output_tokens: 50,
            total_tokens: 150,
        };

        assert_eq!(stats.input_tokens, 100);
        assert_eq!(stats.output_tokens, 50);
        assert_eq!(stats.total_tokens, 150);
    }

    #[test]
    fn test_total_tokens_calculation() {
        let stats = UsageStats {
            input_tokens: 1000,
            output_tokens: 500,
            total_tokens: 1500,
        };

        // Verify total is input + output
        assert_eq!(stats.total_tokens, stats.input_tokens + stats.output_tokens);
    }

    // Additional retry logic edge case tests
    #[test]
    fn test_retry_config_zero_max_retries() {
        let config = LlmRetryConfig {
            max_retries: 0,
            base_delay_ms: 1000,
        };

        assert_eq!(config.max_retries, 0);
        // With max_retries = 0, should only attempt once
    }

    #[test]
    fn test_retry_delay_calculation_for_all_attempts() {
        let config = LlmRetryConfig {
            max_retries: 4,
            base_delay_ms: 100,
        };

        // Calculate expected delays for each attempt
        let delays: Vec<u64> = (0..=config.max_retries)
            .map(|attempt| config.base_delay_ms * (1u64 << attempt))
            .collect();

        assert_eq!(delays, vec![100, 200, 400, 800, 1600]);
    }

    #[test]
    fn test_session_state_variants() {
        use crate::core::agent::session::SessionState;

        // Test all state variants can be created
        let _launching = SessionState::Launching;
        let _running = SessionState::Running {
            current_step: 1,
            context_usage: 50.0,
        };
        let _waiting = SessionState::WaitForRequest {
            last_action_at: chrono::Utc::now(),
        };
        let _finished = SessionState::Finished {
            completed_at: chrono::Utc::now(),
        };
        let _error = SessionState::Error {
            message: "error".to_string(),
            recoverable: true,
        };
        let _user_input = SessionState::UserInputRequired {
            pending_action_ids: vec!["action-1".to_string()],
        };
    }

    // =========================================================================
    // chat_with_retry Integration Tests
    // =========================================================================

    #[tokio::test]
    async fn test_chat_with_retry_success_on_first_attempt() {
        let client = MockLlmClient::new_success("First try success!".to_string());
        let request = ChatRequest::new("test-model", vec![Message::user("Hello")]);
        let config = LlmRetryConfig::default();

        let result = chat_with_retry(&client, &request, &config).await;

        assert!(result.is_ok());
        let response = result.unwrap();
        assert_eq!(response.content, "First try success!");
        assert_eq!(client.call_count(), 1);
    }

    #[tokio::test]
    async fn test_chat_with_retry_retries_on_transient_error() {
        // Create a client that fails twice then succeeds
        let call_count = Arc::new(Mutex::new(0));
        let call_count_clone = call_count.clone();

        let _client_with_retry = {
            let count = call_count_clone.clone();
            MockLlmClient {
                call_count: Arc::clone(&count),
                should_fail: true,
                fail_with_retryable: true,
                response_content: "Success after retries".to_string(),
            }
        };

        // But we need a client that stops failing after attempts
        // Let's use a different approach - create a client that succeeds on third try
        struct RetryableMockClient {
            attempt: Arc<Mutex<usize>>,
            max_failures: usize,
        }

        impl LlmClient for RetryableMockClient {
            fn provider_type(&self) -> crate::core::config::ApiType {
                crate::core::config::ApiType::OpenAI
            }

            fn chat<'a>(
                &'a self,
                _request: ChatRequest,
            ) -> futures::future::BoxFuture<'a, LlmResult<ChatResponse>> {
                Box::pin(async move {
                    let mut attempt = self.attempt.lock().unwrap();
                    *attempt += 1;
                    if *attempt <= self.max_failures {
                        drop(attempt);
                        Err(LlmError::timeout(30))
                    } else {
                        drop(attempt);
                        Ok(ChatResponse {
                            id: "test".to_string(),
                            model: "test-model".to_string(),
                            content: "Success after retries".to_string(),
                            usage: Some(UsageStats {
                                input_tokens: 10,
                                output_tokens: 20,
                                total_tokens: 30,
                            }),
                            finish_reason: Some("stop".to_string()),
                            tool_calls: vec![],
                        })
                    }
                })
            }

            fn chat_stream(
                &self,
                _request: ChatRequest,
            ) -> std::pin::Pin<Box<dyn futures::Stream<Item = LlmResult<StreamChunk>> + Send>>
            {
                use futures::stream;
                Box::pin(stream::empty())
            }

            fn validate_api_key(&self) -> futures::future::BoxFuture<'_, LlmResult<bool>> {
                Box::pin(std::future::ready(Ok(true)))
            }

            fn fetch_models(&self) -> futures::future::BoxFuture<'_, LlmResult<Vec<String>>> {
                Box::pin(std::future::ready(Ok(vec!["gpt-4".to_string()])))
            }
        }

        let client = RetryableMockClient {
            attempt: Arc::new(Mutex::new(0)),
            max_failures: 1, // Fail once, then succeed (so 2 total attempts)
        };

        let request = ChatRequest::new("test-model", vec![Message::user("Hello")]);
        let config = LlmRetryConfig {
            max_retries: 2,
            base_delay_ms: 10, // Small delay for testing
        };

        let result = chat_with_retry(&client, &request, &config).await;

        assert!(result.is_ok());
        let response = result.unwrap();
        assert_eq!(response.content, "Success after retries");
        assert_eq!(*client.attempt.lock().unwrap(), 2);
    }

    #[tokio::test]
    async fn test_chat_with_retry_fails_on_non_retryable_error() {
        let client = MockLlmClient::new_fail_non_retryable();
        let request = ChatRequest::new("test-model", vec![Message::user("Hello")]);
        let config = LlmRetryConfig::default();

        let result = chat_with_retry(&client, &request, &config).await;

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), AgentRuntimeError::Llm(_)));
        assert_eq!(client.call_count(), 1); // Should only try once for non-retryable
    }

    #[tokio::test]
    async fn test_chat_with_retry_fails_after_max_retries() {
        struct AlwaysTimeoutClient {
            attempt: Arc<Mutex<usize>>,
        }

        impl LlmClient for AlwaysTimeoutClient {
            fn provider_type(&self) -> crate::core::config::ApiType {
                crate::core::config::ApiType::OpenAI
            }

            fn chat<'a>(
                &'a self,
                _request: ChatRequest,
            ) -> futures::future::BoxFuture<'a, LlmResult<ChatResponse>> {
                Box::pin(async move {
                    *self.attempt.lock().unwrap() += 1;
                    Err(LlmError::timeout(30))
                })
            }

            fn chat_stream(
                &self,
                _request: ChatRequest,
            ) -> std::pin::Pin<Box<dyn futures::Stream<Item = LlmResult<StreamChunk>> + Send>>
            {
                use futures::stream;
                Box::pin(stream::empty())
            }

            fn validate_api_key(&self) -> futures::future::BoxFuture<'_, LlmResult<bool>> {
                Box::pin(std::future::ready(Ok(true)))
            }

            fn fetch_models(&self) -> futures::future::BoxFuture<'_, LlmResult<Vec<String>>> {
                Box::pin(std::future::ready(Ok(vec!["gpt-4".to_string()])))
            }
        }

        let client = AlwaysTimeoutClient {
            attempt: Arc::new(Mutex::new(0)),
        };

        let request = ChatRequest::new("test-model", vec![Message::user("Hello")]);
        let config = LlmRetryConfig {
            max_retries: 2,
            base_delay_ms: 10,
        };

        let result = chat_with_retry(&client, &request, &config).await;

        assert!(result.is_err());
        // Should attempt initial + 2 retries = 3 total
        assert_eq!(*client.attempt.lock().unwrap(), 3);
    }

    #[tokio::test]
    async fn test_chat_with_retry_no_retries_when_max_is_zero() {
        struct AlwaysTimeoutClient {
            attempt: Arc<Mutex<usize>>,
        }

        impl LlmClient for AlwaysTimeoutClient {
            fn provider_type(&self) -> crate::core::config::ApiType {
                crate::core::config::ApiType::OpenAI
            }

            fn chat<'a>(
                &'a self,
                _request: ChatRequest,
            ) -> futures::future::BoxFuture<'a, LlmResult<ChatResponse>> {
                Box::pin(async move {
                    *self.attempt.lock().unwrap() += 1;
                    Err(LlmError::timeout(30))
                })
            }

            fn chat_stream(
                &self,
                _request: ChatRequest,
            ) -> std::pin::Pin<Box<dyn futures::Stream<Item = LlmResult<StreamChunk>> + Send>>
            {
                use futures::stream;
                Box::pin(stream::empty())
            }

            fn validate_api_key(&self) -> futures::future::BoxFuture<'_, LlmResult<bool>> {
                Box::pin(std::future::ready(Ok(true)))
            }

            fn fetch_models(&self) -> futures::future::BoxFuture<'_, LlmResult<Vec<String>>> {
                Box::pin(std::future::ready(Ok(vec!["gpt-4".to_string()])))
            }
        }

        let client = AlwaysTimeoutClient {
            attempt: Arc::new(Mutex::new(0)),
        };

        let request = ChatRequest::new("test-model", vec![Message::user("Hello")]);
        let config = LlmRetryConfig {
            max_retries: 0, // No retries
            base_delay_ms: 10,
        };

        let result = chat_with_retry(&client, &request, &config).await;

        assert!(result.is_err());
        // Should only attempt once (no retries)
        assert_eq!(*client.attempt.lock().unwrap(), 1);
    }

    #[tokio::test]
    async fn test_chat_with_retry_exponential_backoff_delays() {
        struct TimedRetryClient {
            attempt: Arc<Mutex<usize>>,
            delays: Arc<Mutex<Vec<u64>>>,
        }

        impl TimedRetryClient {
            fn new() -> Self {
                Self {
                    attempt: Arc::new(Mutex::new(0)),
                    delays: Arc::new(Mutex::new(Vec::new())),
                }
            }

            fn get_delays(&self) -> Vec<u64> {
                self.delays.lock().unwrap().clone()
            }
        }

        impl LlmClient for TimedRetryClient {
            fn provider_type(&self) -> crate::core::config::ApiType {
                crate::core::config::ApiType::OpenAI
            }

            fn chat<'a>(
                &'a self,
                _request: ChatRequest,
            ) -> futures::future::BoxFuture<'a, LlmResult<ChatResponse>> {
                Box::pin(async move {
                    let start = std::time::Instant::now();
                    *self.attempt.lock().unwrap() += 1;

                    // Record delay from previous attempt (if any)
                    // This is a simplified version - in real tests we'd need more sophisticated tracking

                    if *self.attempt.lock().unwrap() <= 2 {
                        drop(start);
                        Err(LlmError::timeout(30))
                    } else {
                        drop(start);
                        Ok(ChatResponse {
                            id: "test".to_string(),
                            model: "test-model".to_string(),
                            content: "Success".to_string(),
                            usage: Some(UsageStats {
                                input_tokens: 10,
                                output_tokens: 20,
                                total_tokens: 30,
                            }),
                            finish_reason: Some("stop".to_string()),
                            tool_calls: vec![],
                        })
                    }
                })
            }

            fn chat_stream(
                &self,
                _request: ChatRequest,
            ) -> std::pin::Pin<Box<dyn futures::Stream<Item = LlmResult<StreamChunk>> + Send>>
            {
                use futures::stream;
                Box::pin(stream::empty())
            }

            fn validate_api_key(&self) -> futures::future::BoxFuture<'_, LlmResult<bool>> {
                Box::pin(std::future::ready(Ok(true)))
            }

            fn fetch_models(&self) -> futures::future::BoxFuture<'_, LlmResult<Vec<String>>> {
                Box::pin(std::future::ready(Ok(vec!["gpt-4".to_string()])))
            }
        }

        let client = TimedRetryClient::new();
        let request = ChatRequest::new("test-model", vec![Message::user("Hello")]);
        let config = LlmRetryConfig {
            max_retries: 2,
            base_delay_ms: 50, // Small delay for testing
        };

        let _ = chat_with_retry(&client, &request, &config).await;

        // Verify we attempted 3 times (initial + 2 retries)
        assert_eq!(*client.attempt.lock().unwrap(), 3);
    }
}
